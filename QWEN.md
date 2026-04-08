# smith-rust

## Project Overview

**smith-rust** — это Rust-проект в начальной стадии разработки, предназначенный для создания CLI-приложения с именем `smith`.

Проект сконфигурирован как библиотека (`lib.rs`) и исполняемый файл (`main.rs`), с поддержкой асинхного выполнения через Tokio и возможностью HTTP-запросов через `reqwest`.

### Ключевые особенности

- **CLI-интерфейс** — использует `clap` с derive-макросами для парсинга аргументов командной строки
- **Асинхный рантайм** — Tokio (опционально, включён по умолчанию)
- **HTTP-клиент** — `reqwest` для взаимодействия с внешними API
- **Сериализация** — `serde`, `serde_json`, `serde_yaml` для работы с данными
- **Обработка ошибок** — `thiserror` + `anyhow`
- **Логирование** — `tracing` + `tracing-subscriber` с поддержкой фильтрации через `env-filter`

### Опциональные возможности (features)

| Feature | Описание | Зависимости |
|---------|----------|-------------|
| `runtime-tokio` | Асинхный рантайм (по умолчанию) | `tokio` |
| `mock-llm` | Mock LLM провайдер для тестирования | — |
| `postgres` | Поддержка PostgreSQL | `sqlx` |
| `ratatui` | TUI-интерфейс (в разработке) | `ratatui` |

## Building and Running

```bash
# Собрать проект
cargo build

# Запустить в режиме отладки
cargo run

# Собрать и запустить в release-режиме
cargo run --release

# Запустить тесты
cargo test

# Собрать без дефолтных features
cargo build --no-default-features

# Собрать с PostgreSQL поддержкой
cargo build --features postgres
```

## Project Structure

```
smith-rust/
├── Cargo.toml       # Конфигурация проекта, зависимости, features
├── .gitignore       # Исключает /target
├── src/
│   └── lib.rs       # Библиотека (базовая реализация)
└── target/          # Результаты сборки (игнорируется)
```

> **Примечание:** Файл `src/main.rs` указан в `Cargo.toml`, но ещё не создан.

## Development Conventions

- **Rust Edition:** 2021
- **MSRV:** 1.75
- **Лицензия:** MIT OR Apache-2.0
- **Оптимизация релиза:** LTO = "thin", codegen-units = 1

### Тестирование

В проекте используются:
- `rstest` — фикстуры для тестов
- `mockall` — мок-объекты
- `proptest` — property-based тестирование
- `tokio-test` — утилиты для асинхных тестов
