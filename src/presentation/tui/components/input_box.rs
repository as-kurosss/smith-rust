//! Виджет поля ввода — поддержка редактирования и истории команд.

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

/// Виджет поля ввода.
pub struct InputBoxWidget<'a> {
    input: &'a str,
    cursor_position: usize,
    placeholder: &'a str,
}

impl<'a> InputBoxWidget<'a> {
    /// Создаёт виджет.
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            cursor_position: input.len(),
            placeholder: "Type a message... (Ctrl+C to quit)",
        }
    }

    /// Устанавливает placeholder.
    #[must_use]
    pub fn placeholder(mut self, text: &'a str) -> Self {
        self.placeholder = text;
        self
    }

    /// Устанавливает позицию курсора.
    #[must_use]
    pub fn cursor_position(mut self, pos: usize) -> Self {
        self.cursor_position = pos;
        self
    }
}

impl<'a> Widget for InputBoxWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let display_text = if self.input.is_empty() {
            Span::styled(
                self.placeholder,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(ratatui::style::Modifier::ITALIC),
            )
        } else {
            Span::raw(self.input)
        };

        let paragraph = Paragraph::new(Line::from(display_text));
        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_box_widget_creation() {
        let widget = InputBoxWidget::new("hello");
        assert_eq!(widget.input, "hello");
        assert_eq!(widget.cursor_position, 5);
    }

    #[test]
    fn test_input_box_empty() {
        let widget = InputBoxWidget::new("");
        assert_eq!(widget.input, "");
        assert_eq!(widget.placeholder, "Type a message... (Ctrl+C to quit)");
    }
}
