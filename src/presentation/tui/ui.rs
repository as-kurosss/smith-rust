//! Рендеринг TUI-компонентов.

use ratatui::Frame;

use crate::presentation::tui::app::{calculate_layout, TuiState};
use crate::presentation::tui::components::chat_history::ChatHistoryWidget;
use crate::presentation::tui::components::input_box::InputBoxWidget;
use crate::presentation::tui::components::metrics_panel::{MetricItem, MetricsPanelWidget};
use crate::presentation::tui::components::status_bar::StatusBarWidget;

/// Рендерит весь интерфейс.
pub fn render(frame: &mut Frame<'_>, state: &TuiState) {
    let (status_area, main_area, input_area) = calculate_layout(frame.size());

    // Status bar
    let status_bar = StatusBarWidget {
        llm_status: state.llm_status,
        storage_status: crate::domain::observability::HealthStatus::Healthy,
        memory_status: crate::domain::observability::HealthStatus::Healthy,
        rate_remaining: state.rate_remaining,
        rate_max: state.rate_max,
        session_count: state.session_count,
    };
    frame.render_widget(status_bar, status_area);

    // Main area: chat + metrics sidebar
    let main_chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage(75),
            ratatui::layout::Constraint::Percentage(25),
        ])
        .split(main_area);

    // Chat history
    let chat_widget = ChatHistoryWidget::new(&state.messages);
    frame.render_widget(chat_widget, main_chunks[0]);

    // Metrics panel
    let mut metrics = vec![
        MetricItem {
            label: "Status".to_string(),
            value: format!("{}", state.llm_status),
        },
        MetricItem {
            label: "Rate Limit".to_string(),
            value: format!("{:.0} / {:.0}", state.rate_remaining, state.rate_max),
        },
    ];

    if state.is_thinking {
        metrics.push(MetricItem {
            label: "LLM".to_string(),
            value: "Thinking...".to_string(),
        });
    }

    if let Some(ref err) = state.error_message {
        metrics.push(MetricItem {
            label: "Error".to_string(),
            value: err.clone(),
        });
    }

    let metrics_panel = MetricsPanelWidget {
        metrics,
        recent_tools: state.recent_tools.clone(),
    };
    frame.render_widget(metrics_panel, main_chunks[1]);

    // Input area
    let input_display = if state.is_thinking {
        format!("{} ⏳", state.input)
    } else {
        state.input.clone()
    };
    let input_widget = InputBoxWidget::new(&input_display);
    frame.render_widget(input_widget, input_area);

    // Cursor
    if !state.is_thinking {
        let cursor_x = input_area.x + state.cursor_pos as u16;
        let cursor_y = input_area.y;
        if cursor_x < input_area.x + input_area.width {
            frame.set_cursor(cursor_x, cursor_y);
        }
    }
}
