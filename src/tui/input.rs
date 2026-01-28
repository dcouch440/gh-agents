//! Input bar widget for user text entry.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// Input bar widget displaying prompt, text, and cursor.
pub struct InputBar {
    pub input: String,
    pub cursor_position: usize,
    pub hint: String,
}

impl Default for InputBar {
    fn default() -> Self {
        Self {
            input: String::new(),
            cursor_position: 0,
            hint: "Type /help for commands".to_string(),
        }
    }
}

impl InputBar {
    /// Create a new input bar with the given text.
    pub fn new(input: &str) -> Self {
        let len = input.len();
        Self {
            input: input.to_string(),
            cursor_position: len,
            hint: "Type /help for commands".to_string(),
        }
    }

    /// Set the cursor position.
    pub fn with_cursor(mut self, position: usize) -> Self {
        self.cursor_position = position.min(self.input.len());
        self
    }

    /// Set the hint text.
    pub fn with_hint(mut self, hint: &str) -> Self {
        self.hint = hint.to_string();
        self
    }

    /// Clear the hint.
    pub fn no_hint(mut self) -> Self {
        self.hint.clear();
        self
    }
}

impl Widget for InputBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 {
            return;
        }

        // First line: prompt and input
        let prompt = "> ";
        buf.set_string(area.x, area.y, prompt, Style::default().fg(Color::Green));

        let input_x = area.x + prompt.len() as u16;
        buf.set_string(input_x, area.y, &self.input, Style::default());

        // Cursor indicator (block cursor style)
        let cursor_x = input_x + self.cursor_position as u16;
        if cursor_x < area.right() {
            // If there's a character at cursor, highlight it; otherwise show block
            let cursor_char = self
                .input
                .chars()
                .nth(self.cursor_position)
                .map(|c| c.to_string())
                .unwrap_or_else(|| " ".to_string());

            buf.set_string(
                cursor_x,
                area.y,
                &cursor_char,
                Style::default().fg(Color::Black).bg(Color::White),
            );
        }

        // Second line: hint
        if area.height >= 2 && !self.hint.is_empty() {
            buf.set_string(
                area.x,
                area.y + 1,
                &self.hint,
                Style::default().fg(Color::DarkGray),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_bar_default() {
        let bar = InputBar::default();
        assert!(bar.input.is_empty());
        assert_eq!(bar.cursor_position, 0);
    }

    #[test]
    fn input_bar_new() {
        let bar = InputBar::new("hello");
        assert_eq!(bar.input, "hello");
        assert_eq!(bar.cursor_position, 5);
    }

    #[test]
    fn input_bar_with_cursor() {
        let bar = InputBar::new("hello").with_cursor(2);
        assert_eq!(bar.cursor_position, 2);
    }

    #[test]
    fn input_bar_cursor_clamped() {
        let bar = InputBar::new("hi").with_cursor(100);
        assert_eq!(bar.cursor_position, 2); // clamped to input length
    }

    #[test]
    fn input_bar_with_hint() {
        let bar = InputBar::default().with_hint("Press Enter");
        assert_eq!(bar.hint, "Press Enter");
    }
}
