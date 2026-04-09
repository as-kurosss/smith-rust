//! Санитизация входных данных для безопасного логирования.
//!
//! Маскирует API-ключи, email, токены и другую чувствительную информацию.

use regex::Regex;

/// Маскирует чувствительные данные в строке.
///
/// Паттерны:
/// - API-ключи: `sk-...`, `pk-...`, `api_key=...` → `sk-***`, `pk-***`, `api_key=***`
/// - Email: `user@domain.com` → `u***@***.com`
/// - Bearer-токены: `Bearer eyJ...` → `Bearer ***`
/// - Кредитные карты: последовательности из 13-19 цифр → `****-****-****-1234`
///
/// # Examples
///
/// ```
/// use smith_rust::infrastructure::validation::sanitizer::sanitize_for_logging;
///
/// assert_eq!(sanitize_for_logging("sk-abc123xyz"), "sk-***");
/// assert_eq!(sanitize_for_logging("user@example.com"), "u***@***.com");
/// ```
#[must_use]
pub fn sanitize_for_logging(input: &str) -> String {
    let mut result = input.to_string();

    // API-ключи: sk-xxx, pk-xxx, key-xxx, api_xxx
    result = mask_pattern(&result, r"(?i)(sk|pk|key|api)[_-][a-zA-Z0-9]{3,}", |m| {
        // Find separator and mask everything after it
        let chars: Vec<char> = m.chars().collect();
        if let Some(sep_pos) = chars.iter().position(|&c| c == '-' || c == '_') {
            let prefix: String = chars[..=sep_pos].iter().collect();
            format!("{prefix}***")
        } else {
            "***".to_string()
        }
    });

    // Email
    result = mask_pattern(
        &result,
        r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
        |m| {
            if let Some((local, domain)) = m.split_once('@') {
                if let Some((_, tld)) = domain.rsplit_once('.') {
                    let first_char = local.chars().next().unwrap_or('*');
                    format!("{first_char}***@***.{tld}")
                } else {
                    "***@***.***".to_string()
                }
            } else {
                "***@***.***".to_string()
            }
        },
    );

    // Bearer токены
    result = mask_pattern(&result, r"Bearer\s+[a-zA-Z0-9._-]+", |_m| {
        "Bearer ***".to_string()
    });

    // Номера кредитных карт (13-19 цифр, возможно с разделителями)
    result = mask_pattern(
        &result,
        r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{1,7}\b",
        |m| {
            let digits: String = m.chars().filter(|c| c.is_ascii_digit()).collect();
            let last4 = if digits.len() >= 4 {
                &digits[digits.len() - 4..]
            } else {
                &digits
            };
            format!("****-****-****-{last4}")
        },
    );

    // Пароли в query string: password=xxx
    result = mask_pattern(
        &result,
        r"(?i)(password|passwd|pwd|secret|token)\s*=\s*\S+",
        |m| {
            if let Some(eq_pos) = m.find('=') {
                let key = &m[..=eq_pos];
                format!("{key}***")
            } else {
                "***".to_string()
            }
        },
    );

    result
}

/// Применяет regex-паттерн и заменяет совпадения через callback.
fn mask_pattern(input: &str, pattern: &str, replacer: impl Fn(&str) -> String) -> String {
    let re = match Regex::new(pattern) {
        Ok(re) => re,
        Err(_) => return input.to_string(),
    };
    re.replace_all(input, |caps: &regex::Captures<'_>| {
        // Safety: replace_all guarantees caps.get(0) is Some
        caps.get(0)
            .map(|m| replacer(m.as_str()))
            .unwrap_or_else(|| "***".to_string())
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_api_key_sk() {
        assert_eq!(sanitize_for_logging("sk-abc123xyz"), "sk-***");
    }

    #[test]
    fn test_sanitize_api_key_pk() {
        assert_eq!(sanitize_for_logging("pk-live567"), "pk-***");
    }

    #[test]
    fn test_sanitize_email() {
        let result = sanitize_for_logging("user@example.com");
        assert!(result.starts_with("u***@"));
        assert!(result.ends_with(".com"));
        assert!(!result.contains("user"));
    }

    #[test]
    fn test_sanitize_bearer_token() {
        let result = sanitize_for_logging("Authorization: Bearer eyJhbGciOiJIUzI1NiJ9");
        assert!(result.contains("Bearer ***"));
        assert!(!result.contains("eyJ"));
    }

    #[test]
    fn test_sanitize_credit_card() {
        let result = sanitize_for_logging("card: 4111-1111-1111-1234");
        assert!(result.contains("****-****-****-1234"));
    }

    #[test]
    fn test_sanitize_password_query() {
        let result = sanitize_for_logging("password=secret123");
        assert!(result.contains("password="));
        assert!(!result.contains("secret123"));
    }

    #[test]
    fn test_sanitize_safe_input() {
        let input = "Hello, world! This is a normal message.";
        assert_eq!(sanitize_for_logging(input), input);
    }

    #[test]
    fn test_sanitize_multiple_secrets() {
        let input = "sk-abc123 and user@test.com and Bearer xyz";
        let result = sanitize_for_logging(input);
        assert!(result.contains("sk-***"));
        assert!(result.contains("u***@***.com"));
        assert!(result.contains("Bearer ***"));
    }
}
