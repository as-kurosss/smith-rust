//! OpenAI-совместимый LLM-провайдер.
//!
//! Реализует [`LLMProvider`](crate::domain::LLMProvider) для работы с API,
//! совместимыми с форматом Chat Completions (OpenAI, LocalAI, Ollama и др.).
//!
//! # Rate limiting
//!
//! Встроенный token bucket: 10 запросов/минуту.
//! Для production-использования рассмотреть `governor` crate.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

use crate::domain::message::{LLMResponse, Message};
use crate::domain::LLMProvider as LLMProviderTrait;
use crate::error::{Result, SmithError};

/// Простой token bucket для rate limiting.
///
/// Рефилл: 1 токен каждые 6 секунд (10 запросов/минуту).
/// Для production-использования рассмотреть `governor` crate.
struct TokenBucket {
    /// Текущее количество доступных токенов.
    tokens: f64,
    /// Максимальное количество токенов.
    max_tokens: f64,
    /// Скорость пополнения (токенов в секунду).
    refill_rate: f64,
    /// Время последнего пополнения.
    last_refill: Instant,
}

impl TokenBucket {
    /// Создаёт новый token bucket.
    ///
    /// # Arguments
    ///
    /// * `max_tokens` — макс. количество токенов (ёмкость).
    /// * `refill_rate` — токенов в секунду.
    fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Пополняет бакет на основе прошедшего времени.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    /// Асинхронно ожидает доступный токен.
    ///
    /// Если токенов нет — засыпает до следующего пополнения.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при отмене операции или внутреннем сбое.
    async fn acquire(&mut self, tokens: f64) -> Result<()> {
        loop {
            self.refill();
            if self.tokens >= tokens {
                self.tokens -= tokens;
                debug!(remaining = self.tokens, "token acquired");
                return Ok(());
            }
            // Рассчитываем время до следующего токена
            let deficit = tokens - self.tokens;
            let wait_secs = deficit / self.refill_rate;
            debug!(wait_secs, "waiting for token refill");
            sleep(Duration::from_secs_f64(wait_secs)).await;
        }
    }
}

/// OpenAI-совместимый LLM-провайдер.
///
/// # Example
///
/// ```no_run
/// use smith_rust::infrastructure::llm::openai::OpenAIProvider;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = OpenAIProvider::new(
///     "https://api.openai.com".to_string(),
///     "sk-...".to_string(),
///     "gpt-3.5-turbo".to_string(),
/// )?;
/// # Ok(()) }
/// ```
pub struct OpenAIProvider {
    /// HTTP-клиент (переиспользуется между запросами).
    client: Client,
    /// Base URL API (например, `https://api.openai.com`).
    base_url: String,
    /// API-ключ для аутентификации.
    api_key: String,
    /// Имя модели (например, `gpt-3.5-turbo`).
    model: String,
    /// Rate limiter (token bucket).
    rate_limiter: Arc<Mutex<TokenBucket>>,
}

impl OpenAIProvider {
    /// Создаёт новый экземпляр провайдера.
    ///
    /// # Arguments
    ///
    /// * `base_url` — базовый URL API (без завершающего `/`).
    /// * `api_key` — ключ аутентификации.
    /// * `model` — имя модели.
    ///
    /// Rate limiting: 10 запросов/минуту (рефилл 1 токен каждые 6 секунд).
    ///
    /// # Errors
    ///
    /// Возвращает [`SmithError::LLM`] при невозможности создания HTTP-клиента.
    pub fn new(base_url: String, api_key: String, model: String) -> Result<Self> {
        // 10 requests/minute = 1 token / 6 seconds
        let rate_limiter = Arc::new(Mutex::new(TokenBucket::new(10.0, 1.0 / 6.0)));

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| SmithError::LLM(format!("failed to build HTTP client: {e}")))?;

        Ok(Self {
            client,
            base_url,
            api_key,
            model,
            rate_limiter,
        })
    }

    /// Создаёт экземпляр с кастомным HTTP-клиентом (для тестирования).
    #[cfg(test)]
    fn with_client(client: Client, base_url: String, api_key: String, model: String) -> Self {
        let rate_limiter = Arc::new(Mutex::new(TokenBucket::new(10.0, 1.0 / 6.0)));

        Self {
            client,
            base_url,
            api_key,
            model,
            rate_limiter,
        }
    }

    /// Маппинг доменных [`Message`] в OpenAI-формат.
    fn to_openai_format(messages: &[Message]) -> Vec<OpenAIMessage> {
        messages
            .iter()
            .map(|m| OpenAIMessage {
                role: match m.role {
                    crate::domain::message::MessageRole::User => "user".to_string(),
                    crate::domain::message::MessageRole::Assistant => "assistant".to_string(),
                    crate::domain::message::MessageRole::System => "system".to_string(),
                    crate::domain::message::MessageRole::Tool => "tool".to_string(),
                },
                content: m.content.clone().unwrap_or_default(),
            })
            .collect()
    }

    /// Выполняет HTTP-запрос к API с обработкой статус-кодов и retry.
    ///
    /// Retry логика:
    /// - 5xx → до 2 повторных попыток с exponential backoff
    /// - 429 → SmithError::RateLimited (с парсингом Retry-After)
    /// - 401/403 → SmithError::AuthenticationFailed
    async fn execute_with_retry(&self, payload: &ChatCompletionRequest) -> Result<String> {
        let max_retries = 2;
        let mut attempt = 0;

        loop {
            attempt += 1;
            debug!(attempt, "sending request to OpenAI-compatible API");

            let response = self
                .client
                .request(
                    Method::POST,
                    format!("{}/v1/chat/completions", self.base_url),
                )
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(payload)
                .send()
                .await?;

            let status = response.status();
            let status_code = status.as_u16();
            debug!(status_code, attempt, "received response");

            if status.is_success() {
                let body: ChatCompletionResponse = response.json().await?;
                let content = body
                    .choices
                    .first()
                    .ok_or_else(|| SmithError::LLM("empty choices in response".to_string()))?
                    .message
                    .content
                    .clone();
                return Ok(content);
            }

            // Обработка ошибок — сначала заголовки, потом body
            let retry_after_header = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());

            let body_text = response.text().await.unwrap_or_default();

            match status {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    error!(status_code, "authentication failed");
                    return Err(SmithError::AuthenticationFailed(format!(
                        "status {status_code}: {body_text}"
                    )));
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    warn!(
                        status_code,
                        retry_after = retry_after_header,
                        "rate limited"
                    );
                    return Err(SmithError::RateLimited {
                        retry_after: retry_after_header,
                    });
                }
                s if s.is_server_error() => {
                    warn!(status_code, attempt, body = %body_text, "server error");
                    if attempt >= max_retries {
                        return Err(SmithError::UpstreamError {
                            message: body_text,
                            status_code,
                        });
                    }
                    // Exponential backoff: 1s, 2s
                    let backoff = Duration::from_secs(2_u64.pow(attempt as u32 - 1));
                    info!(backoff_secs = backoff.as_secs(), "retrying after backoff");
                    sleep(backoff).await;
                }
                _ => {
                    error!(status_code, body = %body_text, "unexpected error status");
                    return Err(SmithError::LLM(format!(
                        "unexpected status {status_code}: {body_text}"
                    )));
                }
            }
        }
    }
}

#[async_trait]
impl LLMProviderTrait for OpenAIProvider {
    async fn chat(&self, messages: &[Message]) -> Result<LLMResponse> {
        // Rate limiting
        {
            let mut limiter = self.rate_limiter.lock().await;
            limiter.acquire(1.0).await?;
        }

        let openai_messages = Self::to_openai_format(messages);
        let payload = ChatCompletionRequest {
            model: self.model.clone(),
            messages: openai_messages,
            temperature: 0.7,
            stream: false,
        };

        info!(model = %self.model, message_count = messages.len(), "sending chat request");

        let content = self.execute_with_retry(&payload).await?;

        info!(content_len = content.len(), "chat response received");
        Ok(LLMResponse::new(content))
    }
}

// ===================== API DTOs =====================

/// Сообщение в формате OpenAI Chat Completions.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

/// Запрос к Chat Completions API.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f64,
    stream: bool,
}

/// Ответ от Chat Completions API.
#[derive(Debug, Clone, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Clone, Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Debug, Clone, Deserialize)]
struct AssistantMessage {
    content: String,
}

// ===================== Тесты =====================

#[cfg(test)]
mod tests {
    use super::*;

    use wiremock::matchers::{bearer_token, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new(2.0, 1.0); // 2 tokens, 1/sec
        bucket.acquire(1.0).await.expect("should acquire");
        assert!((bucket.tokens - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_to_openai_format() {
        let messages = vec![Message::system("You are helpful."), Message::user("Hello")];
        let formatted = OpenAIProvider::to_openai_format(&messages);
        assert_eq!(formatted.len(), 2);
        assert_eq!(formatted[0].role, "system");
        assert_eq!(formatted[1].role, "user");
        assert_eq!(formatted[1].content, "Hello");
    }

    #[tokio::test]
    async fn test_openai_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(bearer_token("test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{
                    "message": { "content": "Hello from mock!" }
                }]
            })))
            .mount(&mock_server)
            .await;

        let provider = OpenAIProvider::with_client(
            Client::new(),
            mock_server.uri(),
            "test-api-key".to_string(),
            "gpt-3.5-turbo".to_string(),
        );

        let messages = vec![Message::user("Hi")];
        let response = provider.chat(&messages).await.expect("chat should succeed");
        assert_eq!(response.content, "Hello from mock!");
        assert_eq!(
            response.role,
            crate::domain::message::MessageRole::Assistant
        );
    }

    #[tokio::test]
    async fn test_openai_401_authentication_failed() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": { "message": "Invalid API key" }
            })))
            .mount(&mock_server)
            .await;

        let provider = OpenAIProvider::with_client(
            Client::new(),
            mock_server.uri(),
            "wrong-key".to_string(),
            "gpt-3.5-turbo".to_string(),
        );

        let messages = vec![Message::user("Hi")];
        let result = provider.chat(&messages).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            SmithError::AuthenticationFailed(msg) => {
                assert!(msg.contains("401"));
            }
            e => panic!("Expected AuthenticationFailed, got {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_openai_429_rate_limited() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "30")
                    .set_body_json(serde_json::json!({
                        "error": { "message": "Rate limit exceeded" }
                    })),
            )
            .mount(&mock_server)
            .await;

        let provider = OpenAIProvider::with_client(
            Client::new(),
            mock_server.uri(),
            "test-key".to_string(),
            "gpt-3.5-turbo".to_string(),
        );

        let messages = vec![Message::user("Hi")];
        let result = provider.chat(&messages).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            SmithError::RateLimited { retry_after } => {
                assert_eq!(retry_after, Some(30));
            }
            e => panic!("Expected RateLimited, got {e:?}"),
        }
    }

    #[tokio::test]
    async fn test_openai_500_retry_then_success() {
        let mock_server = MockServer::start().await;

        // Первый запрос — 500, второй — успех
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_json(serde_json::json!({
                        "error": { "message": "Internal error" }
                    }))
                    .set_delay(Duration::from_millis(0)),
            )
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{
                    "message": { "content": "Recovered after 500" }
                }]
            })))
            .mount(&mock_server)
            .await;

        let provider = OpenAIProvider::with_client(
            Client::new(),
            mock_server.uri(),
            "test-key".to_string(),
            "gpt-3.5-turbo".to_string(),
        );

        let messages = vec![Message::user("Hi")];
        let response = provider
            .chat(&messages)
            .await
            .expect("should retry and succeed");
        assert_eq!(response.content, "Recovered after 500");
    }

    #[tokio::test]
    async fn test_openai_500_exhausted_retries() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "error": { "message": "Persistent error" }
            })))
            .mount(&mock_server)
            .await;

        let provider = OpenAIProvider::with_client(
            Client::new(),
            mock_server.uri(),
            "test-key".to_string(),
            "gpt-3.5-turbo".to_string(),
        );

        let messages = vec![Message::user("Hi")];
        let result = provider.chat(&messages).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            SmithError::UpstreamError {
                message,
                status_code,
            } => {
                assert!(message.contains("Persistent error"));
                assert_eq!(status_code, 500);
            }
            e => panic!("Expected UpstreamError, got {e:?}"),
        }
    }
}
