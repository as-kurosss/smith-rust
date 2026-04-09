//! Асинхронный event-loop для TUI.
//!
//! Использует **только** `EventStream` из `crossterm` в комбинации с `tokio::select!`.
//! Все события клавиш фильтруются по `KeyEventKind::Press`, чтобы избежать
//! двойной обработки (KeyDown + KeyUp).

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::{cursor, execute, terminal};
use futures_util::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::domain::message::Message;
use crate::domain::ChatEvent;
use crate::presentation::tui::app::TuiState;
use crate::presentation::tui::ui::render;

/// Запускает TUI с асинхронным event-loop.
///
/// # Errors
///
/// Возвращает ошибку при невозможности инициализации терминала.
pub async fn run_tui(
    mut state: TuiState,
    mut chat_rx: mpsc::Receiver<ChatEvent>,
    tx: mpsc::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Инициализация терминала с alternate screen
    let terminal = setup_terminal()?;

    let result = run_inner(terminal, &mut state, &mut chat_rx, tx).await;

    // Гарантированная очистка (даже при панике)
    cleanup_terminal()?;

    result
}

/// Настраивает терминал: raw mode + alternate screen + hide cursor.
fn setup_terminal(
) -> Result<Terminal<CrosstermBackend<std::io::Stdout>>, Box<dyn std::error::Error + Send + Sync>> {
    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    Ok(terminal)
}

/// Восстанавливает терминал: disable raw mode + leave alternate screen + show cursor.
fn cleanup_terminal() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    terminal::disable_raw_mode()?;
    execute!(
        std::io::stdout(),
        terminal::LeaveAlternateScreen,
        cursor::Show
    )?;
    Ok(())
}

async fn run_inner<B: ratatui::backend::Backend>(
    mut terminal: Terminal<B>,
    state: &mut TuiState,
    chat_rx: &mut mpsc::Receiver<ChatEvent>,
    tx: mpsc::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut event_stream = crossterm::event::EventStream::new();
    let shutdown_signal = tokio::signal::ctrl_c();
    tokio::pin!(shutdown_signal);

    loop {
        // Рендер
        terminal.draw(|frame| render(frame, state))?;

        // Мультиплексирование: пользовательский ввод + chat events + SIGINT
        tokio::select! {
            // 1. OS сигнал (SIGINT / Ctrl+C извне)
            _ = &mut shutdown_signal => {
                info!("Received SIGINT, initiating graceful shutdown");
                break;
            }

            // 2. События терминала (ЕДИНСТВЕННЫЙ источник ввода)
            event = event_stream.next() => {
                match event {
                    Some(Ok(Event::Key(key))) => {
                        // Обрабатываем ТОЛЬКО нажатие (не отпускание) клавиши
                        // Это предотвращает двойной ввод
                        if key.kind == KeyEventKind::Press
                            && handle_key(state, key, &tx).await?
                        {
                            break;
                        }
                    }
                    Some(Ok(Event::Resize(_, _))) => {
                        // Terminal resized, ratatui handles layout on next draw
                    }
                    Some(Err(e)) => {
                        warn!(error = %e, "event stream error");
                    }
                    None => {
                        // Stream closed
                        break;
                    }
                    _ => {}
                }
            }

            // 3. Обновления от бизнес-логики
            event = chat_rx.recv() => {
                match event {
                    Some(chat_event) => handle_chat_event(state, chat_event),
                    None => {
                        // Channel closed
                        break;
                    }
                }
            }
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}

async fn handle_key(
    state: &mut TuiState,
    key: KeyEvent,
    tx: &mpsc::Sender<String>,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    match (key.modifiers, key.code) {
        // Ctrl+C — выход (явный перехват, т.к. raw mode блокирует SIGINT)
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            info!("Ctrl+C pressed, shutting down");
            state.should_quit = true;
            return Ok(true);
        }
        // Esc — выход
        (_, KeyCode::Esc) => {
            info!("Esc pressed, shutting down");
            state.should_quit = true;
            return Ok(true);
        }
        // Enter — отправка
        (_, KeyCode::Enter) => {
            if let Some(input) = state.submit_input() {
                info!(input = %input, "TUI: user submitted message");
                if let Err(e) = tx.send(input).await {
                    error!(error = %e, "TUI: failed to send message");
                    state.error_message = Some("Failed to send message".to_string());
                }
            }
        }
        // Backspace
        (_, KeyCode::Backspace) => {
            state.handle_backspace();
        }
        // Delete
        (_, KeyCode::Delete) => {
            state.handle_delete();
        }
        // Left
        (_, KeyCode::Left) => {
            state.move_cursor_left();
        }
        // Right
        (_, KeyCode::Right) => {
            state.move_cursor_right();
        }
        // Up — история
        (_, KeyCode::Up) => {
            state.navigate_history_up();
        }
        // Down — история
        (_, KeyCode::Down) => {
            state.navigate_history_down();
        }
        // Home
        (_, KeyCode::Home) => {
            state.cursor_pos = 0;
        }
        // End
        (_, KeyCode::End) => {
            state.cursor_pos = state.input.chars().count();
        }
        // Tab — автодополнение (заглушка)
        (_, KeyCode::Tab) => {
            // TODO: implement tab completion
        }
        // Char — только без modifier-ов (иначе Ctrl+X и т.д. тоже попадут)
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            state.handle_char(c);
        }
        _ => {}
    }

    Ok(false)
}

fn handle_chat_event(state: &mut TuiState, event: ChatEvent) {
    match event {
        ChatEvent::UserMessage { content, .. } => {
            state.add_message(Message::user(content));
        }
        ChatEvent::AssistantMessage { content, .. } => {
            state.add_message(Message::assistant(content));
            state.set_thinking(false);
        }
        ChatEvent::ToolCall {
            tool_name,
            arguments,
            ..
        } => {
            state.set_thinking(true);
            state.add_tool_call(tool_name.clone(), arguments, true);
        }
        ChatEvent::ToolResult {
            tool_name,
            content,
            success,
            ..
        } => {
            state.add_tool_call(tool_name.clone(), content.clone(), success);
            state.add_message(Message::tool_result("tool", tool_name, content));
        }
        ChatEvent::Error { message, .. } => {
            state.error_message = Some(message);
            state.set_thinking(false);
        }
        ChatEvent::Thinking { thinking, .. } => {
            state.set_thinking(thinking);
        }
    }
}
