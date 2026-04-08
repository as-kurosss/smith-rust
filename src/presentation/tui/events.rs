//! Асинхронный event-loop для TUI.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use futures_util::StreamExt;
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
    // Инициализация terminal
    crossterm::terminal::enable_raw_mode()?;
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stderr());
    let mut terminal = ratatui::Terminal::new(backend)?;
    terminal.clear()?;

    let result = run_inner(&mut terminal, &mut state, &mut chat_rx, tx).await;

    // Cleanup
    crossterm::terminal::disable_raw_mode()?;
    terminal.show_cursor()?;

    result
}

async fn run_inner<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    state: &mut TuiState,
    chat_rx: &mut mpsc::Receiver<ChatEvent>,
    tx: mpsc::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut event_reader = crossterm::event::EventStream::new();

    loop {
        // Рендер
        terminal.draw(|frame| render(frame, state))?;

        // Мультиплексирование: пользовательский ввод + chat events
        tokio::select! {
            // Пользовательский ввод
            event = event_reader.next() => {
                match event {
                    Some(Ok(Event::Key(key))) => {
                        if handle_key(state, key, &tx).await? {
                            break;
                        }
                    }
                    Some(Ok(Event::Resize(_, _))) => {
                        // Terminal resized, will re-render
                    }
                    Some(Err(e)) => {
                        warn!(error = %e, "event read error");
                    }
                    None => {
                        // Channel closed
                        break;
                    }
                    _ => {}
                }
            }
            // Chat events
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
        // Ctrl+C — выход
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            state.should_quit = true;
            return Ok(true);
        }
        // Esc — выход
        (_, KeyCode::Esc) => {
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
            state.cursor_pos = state.input.len();
        }
        // Char
        (_, KeyCode::Char(c)) => {
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
