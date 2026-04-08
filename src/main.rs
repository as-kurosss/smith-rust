//! Точка входа бинарного приложения `smith`.
//!
//! Тонкая обёртка:
//! 1. Парсит CLI-аргументы.
//! 2. Инициализирует tracing.
//! 3. Запускает tokio runtime.
//! 4. Создаёт ChatSession с mock-провайдером.
//! 5. Передаёт управление в `run_chat_loop`.

use std::io::{self, BufReader};

use anyhow::{Context, Result};
use tracing::info;

use smith_rust::{init_tracing, run_chat_loop, ChatConfig, ChatSession, CliArgs, MockLLMProvider};

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Парсим CLI-аргументы
    let args = CliArgs::parse_from_cli();

    // 2. Инициализируем логирование
    init_tracing(&args.log_level);
    info!(version = env!("CARGO_PKG_VERSION"), "smith started");

    // 3. Создаём конфигурацию чата
    let config = ChatConfig {
        max_history: args.max_history,
        system_prompt: Some(args.system_prompt),
    };

    // 4. Выбираем провайдер (mock-режим по умолчанию для шага 00)
    let provider = if args.mock {
        info!("using mock LLM provider");
        Box::new(MockLLMProvider::new()) as Box<dyn smith_rust::LLMProvider>
    } else {
        anyhow::bail!("real LLM providers are not yet implemented — use --mock flag");
    };

    // 5. Создаём сессию
    let mut session = ChatSession::new(provider, config);
    info!("chat session initialized");

    // 6. Запускаем интерактивный цикл
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
