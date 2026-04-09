//! Управление состоянием TUI-интерфейса.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::domain::message::Message;
use crate::domain::observability::HealthStatus;

/// Состояние TUI-приложения.
#[derive(Debug)]
pub struct TuiState {
    /// История сообщений.
    pub messages: Vec<Message>,
    /// Текущий ввод.
    pub input: String,
    /// Позиция курсора.
    pub cursor_pos: usize,
    /// Статус LLM.
    pub llm_status: HealthStatus,
    /// Оставшиеся токены rate limit.
    pub rate_remaining: f64,
    /// Максимум токенов rate limit.
    pub rate_max: f64,
    /// Флаг "думаю...".
    pub is_thinking: bool,
    /// Количество сессий.
    pub session_count: usize,
    /// Последние вызовы инструментов: (name, status, success).
    pub recent_tools: Vec<(String, String, bool)>,
    /// Флаг выхода.
    pub should_quit: bool,
    /// История введённых команд (для навигации ↑/↓).
    pub input_history: Vec<String>,
    /// Текущая позиция в истории ввода.
    pub history_index: Option<usize>,
    /// Сообщение об ошибке (для отображения).
    pub error_message: Option<String>,
}

impl TuiState {
    /// Создаёт новое состояние.
    #[must_use]
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor_pos: 0,
            llm_status: HealthStatus::Healthy,
            rate_remaining: 60.0,
            rate_max: 60.0,
            is_thinking: false,
            session_count: 0,
            recent_tools: Vec::new(),
            should_quit: false,
            input_history: Vec::new(),
            history_index: None,
            error_message: None,
        }
    }

    /// Добавляет сообщение в историю.
    pub fn add_message(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    /// Устанавливает статус LLM.
    pub fn set_llm_status(&mut self, status: HealthStatus) {
        self.llm_status = status;
    }

    /// Обновляет rate limit.
    pub fn update_rate_limit(&mut self, remaining: f64, max: f64) {
        self.rate_remaining = remaining;
        self.rate_max = max;
    }

    /// Добавляет инструмент в список последних.
    pub fn add_tool_call(&mut self, name: String, status: String, success: bool) {
        self.recent_tools.push((name, status, success));
        // Оставляем только последние 5
        if self.recent_tools.len() > 5 {
            self.recent_tools.drain(0..self.recent_tools.len() - 5);
        }
    }

    /// Устанавливает флаг "думаю".
    pub fn set_thinking(&mut self, thinking: bool) {
        self.is_thinking = thinking;
    }

    /// Обрабатывает ввод символа.
    pub fn handle_char(&mut self, c: char) {
        let byte_idx = self
            .input
            .char_indices()
            .nth(self.cursor_pos)
            .map(|(i, _)| i)
            .unwrap_or(self.input.len());
        self.input.insert(byte_idx, c);
        self.cursor_pos += 1;
        self.history_index = None;
        self.error_message = None;
    }

    /// Обрабатывает backspace.
    pub fn handle_backspace(&mut self) {
        if self.cursor_pos > 0 {
            let byte_idx = self
                .input
                .char_indices()
                .nth(self.cursor_pos - 1)
                .map(|(i, _)| i)
                .unwrap_or(self.input.len());
            self.input.remove(byte_idx);
            self.cursor_pos -= 1;
        }
    }

    /// Обрабатывает delete.
    pub fn handle_delete(&mut self) {
        let char_count = self.input.chars().count();
        if self.cursor_pos < char_count {
            let byte_idx = self
                .input
                .char_indices()
                .nth(self.cursor_pos)
                .map(|(i, _)| i)
                .unwrap_or(self.input.len());
            self.input.remove(byte_idx);
        }
    }

    /// Перемещает курсор влево.
    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// Перемещает курсор вправо.
    pub fn move_cursor_right(&mut self) {
        let char_count = self.input.chars().count();
        if self.cursor_pos < char_count {
            self.cursor_pos += 1;
        }
    }

    /// Переходит к предыдущему элементу в истории ввода.
    pub fn navigate_history_up(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        let new_index = match self.history_index {
            None => self.input_history.len() - 1,
            Some(idx) if idx > 0 => idx - 1,
            Some(_) => 0,
        };
        self.history_index = Some(new_index);
        self.input = self.input_history[new_index].clone();
        self.cursor_pos = self.input.chars().count();
    }

    /// Переходит к следующему элементу в истории ввода.
    pub fn navigate_history_down(&mut self) {
        match self.history_index {
            None => {}
            Some(idx) if idx + 1 >= self.input_history.len() => {
                self.history_index = None;
                self.input.clear();
                self.cursor_pos = 0;
            }
            Some(idx) => {
                let new_idx = idx + 1;
                self.history_index = Some(new_idx);
                self.input = self.input_history[new_idx].clone();
                self.cursor_pos = self.input.chars().count();
            }
        }
    }

    /// Подтверждает ввод и возвращает строку.
    pub fn submit_input(&mut self) -> Option<String> {
        let input = self.input.trim().to_string();
        if input.is_empty() {
            return None;
        }
        self.input_history.push(input.clone());
        self.input.clear();
        self.cursor_pos = 0;
        self.history_index = None;
        Some(input)
    }
}

impl Default for TuiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Рассчитывает layout области TUI.
///
/// Возвращает: (status_bar, main_area, input_area).
#[must_use]
pub fn calculate_layout(area: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // status bar
            Constraint::Min(1),    // main chat area
            Constraint::Length(3), // input area
        ])
        .split(area);

    (chunks[0], chunks[1], chunks[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_state_new() {
        let state = TuiState::new();
        assert!(!state.should_quit);
        assert!(state.messages.is_empty());
        assert_eq!(state.rate_max, 60.0);
    }

    #[test]
    fn test_handle_char_and_submit() {
        let mut state = TuiState::new();
        state.handle_char('h');
        state.handle_char('i');
        let result = state.submit_input();
        assert_eq!(result, Some("hi".to_string()));
        assert!(state.input.is_empty());
    }

    #[test]
    fn test_handle_backspace() {
        let mut state = TuiState::new();
        state.handle_char('a');
        state.handle_char('b');
        state.handle_backspace();
        assert_eq!(state.input, "a");
    }

    #[test]
    fn test_history_navigation() {
        let mut state = TuiState::new();
        state.input = "msg1".to_string();
        state.submit_input();
        state.input = "msg2".to_string();
        state.submit_input();

        state.navigate_history_up();
        assert_eq!(state.input, "msg2");
        state.navigate_history_up();
        assert_eq!(state.input, "msg1");
        state.navigate_history_down();
        assert_eq!(state.input, "msg2");
        state.navigate_history_down();
        assert!(state.input.is_empty());
    }

    #[test]
    fn test_calculate_layout() {
        let area = Rect::new(0, 0, 80, 24);
        let (status, main, input) = calculate_layout(area);
        assert_eq!(status.height, 1);
        assert_eq!(input.height, 3);
        // Layout chunks may have small rounding differences
        assert!(status.y + main.height + input.height >= 20);
    }

    #[test]
    fn test_add_tool_call_limit() {
        let mut state = TuiState::new();
        for i in 0..10 {
            state.add_tool_call(format!("tool{i}"), "ok".to_string(), true);
        }
        assert!(state.recent_tools.len() <= 5);
    }

    #[test]
    fn test_unicode_input_cyrillic() {
        let mut state = TuiState::new();
        // Вводим кириллицу: "Пр"
        state.handle_char('П');
        state.handle_char('р');
        assert_eq!(state.input, "Пр");
        assert_eq!(state.cursor_pos, 2); // 2 символа

        // Backspace удаляет 'р'
        state.handle_backspace();
        assert_eq!(state.input, "П");
        assert_eq!(state.cursor_pos, 1);

        // Ещё backspace
        state.handle_backspace();
        assert_eq!(state.input, "");
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn test_unicode_cursor_navigation() {
        let mut state = TuiState::new();
        state.handle_char('п');
        state.handle_char('р');
        state.handle_char('и');
        state.handle_char('в');
        state.handle_char('е');
        state.handle_char('т');
        assert_eq!(state.cursor_pos, 6); // 6 символов

        // Стрелка влево
        state.move_cursor_left();
        assert_eq!(state.cursor_pos, 5);

        // Вставляем символ в середину
        state.handle_char('!');
        assert_eq!(state.input, "приве!т");
        assert_eq!(state.cursor_pos, 6);

        // Delete удаляет 'т'
        state.handle_delete();
        assert_eq!(state.input, "приве!");
    }

    #[test]
    fn test_unicode_emoji() {
        let mut state = TuiState::new();
        state.handle_char('🚀');
        state.handle_char('🔥');
        assert_eq!(state.input, "🚀🔥");
        assert_eq!(state.cursor_pos, 2); // 2 символа (но 8 байт)

        state.move_cursor_left();
        assert_eq!(state.cursor_pos, 1);
        state.handle_char('💡');
        assert_eq!(state.input, "🚀💡🔥");
    }
}
