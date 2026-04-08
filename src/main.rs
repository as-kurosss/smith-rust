//! Точка входа бинарного приложения `smith`.
//!
//! Поддерживает два режима:
//! - CLI (по умолчанию) — интерактивный цикл через stdin/stdout
//! - TUI (флаг `--tui`) — ratatui-based терминальный UI

use std::io::{self, BufReader};

use anyhow::{Context, Result};
use tokio::sync::mpsc;
use tracing::{error, info};

use smith_rust::{
    init_tracing, run_chat_loop, ChatConfig, ChatSession, CliArgs, MockLLMProvider, ToolRegistry,
};

#[cfg(feature = "tui")]
use smith_rust::{run_tui, ChatEvent, TuiState};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Парсим CLI-аргументы
    let args = CliArgs::parse_from_cli();

    // 2. Инициализируем логирование
    init_tracing(&args.log_level);
    info!(version = env!("CARGO_PKG_VERSION"), "smith started");

    // 3. Создаём конфигурацию чата
    let tool_registry = std::sync::Arc::new(ToolRegistry::default_tools());
    let config = ChatConfig {
        max_history: args.max_history,
        system_prompt: Some(args.system_prompt),
        tool_registry: Some(tool_registry),
        max_tool_iterations: 5,
    };

    // 4. Выбираем провайдер
    let provider = if args.mock {
        info!("using mock LLM provider");
        Box::new(MockLLMProvider::new()) as Box<dyn smith_rust::LLMProvider>
    } else {
        anyhow::bail!("real LLM providers are not yet implemented — use --mock flag");
    };

    // 5. Запускаем нужный режим
    #[cfg(feature = "tui")]
    {
        // Проверяем, есть ли флаг --tui в аргументах
        let use_tui = std::env::args().any(|a| a == "--tui");
        if use_tui {
            return run_tui_mode(config, provider).await;
        }
    }

    // CLI режим
    run_cli_mode(config, provider).await
}

/// Запускает CLI режим.
async fn run_cli_mode(
    config: ChatConfig,
    provider: Box<dyn smith_rust::LLMProvider>,
) -> Result<()> {
    let mut session = ChatSession::new(provider, config);
    info!("chat session initialized");

    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let writer = io::LineWriter::new(stdout.lock());

    run_chat_loop(&mut session, reader, writer)
        .await
        .context("chat loop failed")?;

    info!("smith exited gracefully");
    Ok(())
}

/// Запускает TUI режим.
#[cfg(feature = "tui")]
async fn run_tui_mode(
    config: ChatConfig,
    provider: Box<dyn smith_rust::LLMProvider>,
) -> Result<()> {
    info!("starting TUI mode");

    let (chat_tx, chat_rx) = mpsc::channel::<ChatEvent>(32);
    let (user_tx, mut user_rx) = mpsc::channel::<String>(8);

    let state = TuiState::new();
    let config_clone = config.clone();

    // Запускаем chat_loop в фоне
    let chat_handle = tokio::spawn(async move {
        let mut session = ChatSession::new(provider, config_clone);
        loop {
            tokio::select! {
                Some(input) = user_rx.recv() => {
                    // Отправляем событие user message
                    let _ = chat_tx.send(ChatEvent::UserMessage {
                        session_id: uuid::Uuid::new_v4(),
                        content: input.clone(),
                        timestamp: chrono::Utc::now(),
                    }).await;

                    // Thinking
                    let _ = chat_tx.send(ChatEvent::Thinking {
                        session_id: uuid::Uuid::new_v4(),
                        thinking: true,
                    }).await;

                    match session.process_message(&input).await {
                        Ok(response) => {
                            let _ = chat_tx.send(ChatEvent::AssistantMessage {
                                session_id: uuid::Uuid::new_v4(),
                                content: response.content,
                                timestamp: chrono::Utc::now(),
                            }).await;
                            let _ = chat_tx.send(ChatEvent::Thinking {
                                session_id: uuid::Uuid::new_v4(),
                                thinking: false,
                            }).await;
                        }
                        Err(e) => {
                            let _ = chat_tx.send(ChatEvent::Error {
                                session_id: uuid::Uuid::new_v4(),
                                message: format!("{e}"),
                                timestamp: chrono::Utc::now(),
                            }).await;
                        }
                    }
                }
                else => break,
            }
        }
    });

    // Запускаем TUI
    let tui_result = run_tui(state, chat_rx, user_tx).await;

    chat_handle.abort();

    if let Err(e) = tui_result {
        error!(error = %e, "TUI failed");
        // Fallback: если TUI не запустился, пытаемся CLI
        eprintln!("TUI initialization failed: {e}. Falling back to CLI.");
        let mut session = ChatSession::new(MockLLMProvider::new(), config);
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        let stdout = io::stdout();
        let writer = io::LineWriter::new(stdout.lock());
        run_chat_loop(&mut session, reader, writer)
            .await
            .context("chat loop failed")?;
    }
    info!("TUI mode exited");
    Ok(())
}
