//! Виджет боковой панели — метрики и активные инструменты.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

/// Элемент боковой панели.
#[derive(Debug, Clone)]
pub struct MetricItem {
    pub label: String,
    pub value: String,
}

/// Виджет боковой панели с метриками.
pub struct MetricsPanelWidget {
    pub metrics: Vec<MetricItem>,
    pub recent_tools: Vec<(String, String, bool)>, // (name, status, success)
}

impl Widget for MetricsPanelWidget {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let mut lines = Vec::new();

        // Заголовок
        lines.push(Line::from(vec![Span::styled(
            "── Metrics ──",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));

        for item in &self.metrics {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}: ", item.label),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(&item.value),
            ]));
        }

        lines.push(Line::from(""));

        // Активные инструменты
        lines.push(Line::from(vec![Span::styled(
            "── Recent Tools ──",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));

        if self.recent_tools.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (none)",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for item in self.recent_tools.iter().take(3) {
                let (ref name, ref status, success) = *item;
                let icon = if success { "✓" } else { "✗" };
                let color = if success { Color::Green } else { Color::Red };
                lines.push(Line::from(vec![
                    Span::styled(format!(" {icon} "), Style::default().fg(color)),
                    Span::raw(name.clone()),
                    Span::styled(format!(" ({status})"), Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        let block = Block::default()
            .borders(Borders::LEFT)
            .style(Style::default().fg(Color::DarkGray));

        Paragraph::new(lines).block(block).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_panel_widget_creation() {
        let widget = MetricsPanelWidget {
            metrics: vec![MetricItem {
                label: "Requests".to_string(),
                value: "42".to_string(),
            }],
            recent_tools: vec![("calculator".to_string(), "success".to_string(), true)],
        };
        assert_eq!(widget.metrics.len(), 1);
        assert_eq!(widget.recent_tools.len(), 1);
    }

    #[test]
    fn test_metrics_panel_empty() {
        let widget = MetricsPanelWidget {
            metrics: vec![],
            recent_tools: vec![],
        };
        assert!(widget.metrics.is_empty());
        assert!(widget.recent_tools.is_empty());
    }
}
