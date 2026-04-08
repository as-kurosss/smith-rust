//! Виджет статусной строки — отображение здоровья и rate limit.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::domain::observability::HealthStatus;

/// Виджет статусной строки.
pub struct StatusBarWidget {
    pub llm_status: HealthStatus,
    pub storage_status: HealthStatus,
    pub memory_status: HealthStatus,
    pub rate_remaining: f64,
    pub rate_max: f64,
    pub session_count: usize,
}

impl Widget for StatusBarWidget {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let status_color = match (self.llm_status, self.storage_status, self.memory_status) {
            (HealthStatus::Unhealthy, _, _)
            | (_, HealthStatus::Unhealthy, _)
            | (_, _, HealthStatus::Unhealthy) => Color::Red,
            (HealthStatus::Degraded, _, _)
            | (_, HealthStatus::Degraded, _)
            | (_, _, HealthStatus::Degraded) => Color::Yellow,
            _ => Color::Green,
        };

        let status_icon = match (self.llm_status, self.storage_status, self.memory_status) {
            (HealthStatus::Unhealthy, _, _)
            | (_, HealthStatus::Unhealthy, _)
            | (_, _, HealthStatus::Unhealthy) => "❌",
            (HealthStatus::Degraded, _, _)
            | (_, HealthStatus::Degraded, _)
            | (_, _, HealthStatus::Degraded) => "⚠️ ",
            _ => "✅",
        };

        let rate_pct = if self.rate_max > 0.0 {
            (self.rate_remaining / self.rate_max * 100.0) as u32
        } else {
            0
        };

        let lines = vec![Line::from(vec![
            Span::raw(status_icon),
            Span::styled(
                format!(" LLM: {} ", self.llm_status),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "Rate: {:.0}/{} ({}%) ",
                    self.rate_remaining, self.rate_max as u32, rate_pct
                ),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("Sessions: {}", self.session_count),
                Style::default().fg(Color::DarkGray),
            ),
        ])];

        Paragraph::new(lines).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_bar_widget_creation() {
        let widget = StatusBarWidget {
            llm_status: HealthStatus::Healthy,
            storage_status: HealthStatus::Healthy,
            memory_status: HealthStatus::Healthy,
            rate_remaining: 50.0,
            rate_max: 60.0,
            session_count: 3,
        };
        assert_eq!(widget.rate_remaining, 50.0);
        assert_eq!(widget.session_count, 3);
    }
}
