//! Виджет истории чата — отображение сообщений пользователя и ассистента.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::domain::message::{Message, MessageRole};

/// Виджет для отображения истории чата.
pub struct ChatHistoryWidget<'a> {
    messages: &'a [Message],
    max_lines: usize,
}

impl<'a> ChatHistoryWidget<'a> {
    /// Создаёт виджет из истории сообщений.
    #[must_use]
    pub fn new(messages: &'a [Message]) -> Self {
        Self {
            messages,
            max_lines: 1000,
        }
    }

    /// Устанавливает максимальное количество отображаемых строк.
    #[must_use]
    pub fn max_lines(mut self, max: usize) -> Self {
        self.max_lines = max;
        self
    }

    /// Конвертирует сообщения в строки для рендеринга.
    #[must_use]
    pub fn to_lines(&self) -> Vec<Line<'a>> {
        let mut lines = Vec::new();
        for msg in self.messages {
            let content = msg.content_or_empty();
            let (label, color) = match msg.role {
                MessageRole::User => ("You", Color::Green),
                MessageRole::Assistant => ("Assistant", Color::Cyan),
                MessageRole::System => ("System", Color::Yellow),
                MessageRole::Tool => ("Tool", Color::Magenta),
            };

            // Заголовок сообщения
            lines.push(Line::from(vec![
                Span::styled(
                    format!("[{label}] "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(content.to_string(), Style::default().fg(Color::White)),
            ]));

            // Tool call info
            if let Some(tool_calls) = &msg.tool_calls {
                for tc in tool_calls {
                    lines.push(Line::from(vec![
                        Span::styled(
                            "  ⚙ Tool: ",
                            Style::default()
                                .fg(Color::Magenta)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(&tc.function.name),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("    Args: ", Style::default().fg(Color::DarkGray)),
                        Span::raw(&tc.function.arguments),
                    ]));
                }
            }

            // Имя инструмента для tool result
            if msg.role == MessageRole::Tool {
                if let Some(ref name) = msg.name {
                    lines.push(Line::from(vec![Span::styled(
                        format!("  ↳ {name}"),
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::DIM),
                    )]));
                }
            }

            lines.push(Line::from("")); // Пустая строка-разделитель
        }

        // Ограничиваем количество строк
        if lines.len() > self.max_lines {
            lines.drain(0..lines.len() - self.max_lines);
        }

        lines
    }
}

impl<'a> Widget for ChatHistoryWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines = self.to_lines();
        let paragraph = Paragraph::new(lines).scroll((0, 0));
        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_history_to_lines() {
        let messages = vec![Message::user("hello"), Message::assistant("hi there")];
        let widget = ChatHistoryWidget::new(&messages);
        let lines = widget.to_lines();
        // Каждый message = 2 строки (label+content + empty)
        assert!(lines.len() >= 4);
    }

    #[test]
    fn test_chat_history_empty() {
        let messages: Vec<Message> = vec![];
        let widget = ChatHistoryWidget::new(&messages);
        let lines = widget.to_lines();
        assert!(lines.is_empty());
    }

    #[test]
    fn test_chat_history_max_lines() {
        let messages: Vec<Message> = (0..500)
            .map(|i| Message::user(format!("msg {i}")))
            .collect();
        let widget = ChatHistoryWidget::new(&messages).max_lines(10);
        let lines = widget.to_lines();
        assert!(lines.len() <= 10);
    }
}
