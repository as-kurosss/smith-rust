/// Интеграционный тест полного цикла чата.
///
/// Тестирует взаимодействие ChatSession с MockLLMProvider
/// через интерфейс run_chat_loop.
use std::io::{BufReader, Cursor, LineWriter};

use smith_rust::{run_chat_loop, ChatConfig, ChatSession, MockLLMProvider};

#[tokio::test]
async fn test_full_chat_cycle_with_mock() {
    // Входные данные: одно сообщение + exit для завершения
    let input = "Hello, world!\nexit\n";
    let reader = BufReader::new(Cursor::new(input));
    let writer = Cursor::new(Vec::new());
    let writer = LineWriter::new(writer);

    let provider = MockLLMProvider::new();
    let config = ChatConfig::default();
    let mut session = ChatSession::new(provider, config);

    let result = run_chat_loop(&mut session, reader, writer).await;
    assert!(result.is_ok(), "chat loop should complete successfully");
}

#[tokio::test]
async fn test_multiple_messages() {
    let input = "First message\nSecond message\nquit\n";
    let reader = BufReader::new(Cursor::new(input));
    let writer = Cursor::new(Vec::new());
    let writer = LineWriter::new(writer);

    let provider = MockLLMProvider::new();
    let config = ChatConfig::default();
    let mut session = ChatSession::new(provider, config);

    run_chat_loop(&mut session, reader, writer)
        .await
        .expect("should handle multiple messages");

    // Проверяем, что история содержит user+assistant пары
    let history = session.history();
    // system + 2 * (user + assistant) = 5
    assert_eq!(history.len(), 5);
}

#[tokio::test]
async fn test_empty_lines_skipped() {
    let input = "\n\nActual message\n\nexit\n";
    let reader = BufReader::new(Cursor::new(input));
    let writer = Cursor::new(Vec::new());
    let writer = LineWriter::new(writer);

    let provider = MockLLMProvider::new();
    let config = ChatConfig::default();
    let mut session = ChatSession::new(provider, config);

    run_chat_loop(&mut session, reader, writer)
        .await
        .expect("empty lines should be skipped");

    // Только system + 1 пользовательское сообщение + 1 ответ
    let history = session.history();
    assert_eq!(history.len(), 3);
}

#[tokio::test]
async fn test_error_recovery() {
    // Пустая строка (после trim) должна вызвать ошибку, но цикл продолжается
    let input = "   \nValid message\nexit\n";
    let reader = BufReader::new(Cursor::new(input));
    let writer = Cursor::new(Vec::new());
    let writer = LineWriter::new(writer);

    let provider = MockLLMProvider::new();
    let config = ChatConfig::default();
    let mut session = ChatSession::new(provider, config);

    // "   " (пробелы) после trim = empty -> ошибка
    // "Valid message" -> успех
    run_chat_loop(&mut session, reader, writer)
        .await
        .expect("should recover from empty input error");

    let history = session.history();
    // system + Valid message (user + assistant) = 3
    assert_eq!(history.len(), 3);
}

#[cfg(feature = "mock-llm")]
#[tokio::test]
async fn test_mock_response_format() {
    use smith_rust::LLMProvider;

    let provider = MockLLMProvider::new();
    let messages = vec![smith_rust::Message::user("test input")];
    let response = provider.chat(&messages).await.expect("should succeed");

    // Проверяем формат mock-ответа
    assert!(
        response.content.starts_with("[MOCK] Response to:"),
        "mock response should start with [MOCK] prefix"
    );
    assert!(
        response.content.contains("test input"),
        "mock response should contain user input"
    );
}
