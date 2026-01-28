//! Layout system for the TUI.
//!
//! Provides a fixed three-part layout:
//! - Header bar (1 line): Agent status and current view
//! - Main area (flexible): Content for the current view
//! - Input bar (2 lines): User input and hints

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::Widget,
};

/// Fixed application layout with header, main area, and input bar.
pub struct AppLayout {
    pub header: Rect,
    pub main: Rect,
    pub input: Rect,
}

impl AppLayout {
    /// Create a new layout for the given area.
    pub fn new(area: Rect) -> Self {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Min(1),    // main content
                Constraint::Length(2), // input bar
            ])
            .split(area);

        Self {
            header: chunks[0],
            main: chunks[1],
            input: chunks[2],
        }
    }
}

/// Header bar showing agent status and current view.
pub struct HeaderBar {
    pub workers_active: u8,
    pub workers_total: u8,
    pub orchestrators_active: u8,
    pub orchestrators_total: u8,
    pub utilities_active: u8,
    pub utilities_total: u8,
    pub current_view: String,
    pub mode_indicator: String,
}

impl Default for HeaderBar {
    fn default() -> Self {
        Self {
            workers_active: 0,
            workers_total: 6,
            orchestrators_active: 0,
            orchestrators_total: 2,
            utilities_active: 0,
            utilities_total: 4,
            current_view: "/home".to_string(),
            mode_indicator: String::new(),
        }
    }
}

impl HeaderBar {
    /// Create a new header bar with agent counts.
    pub fn new(
        workers_active: u8,
        workers_total: u8,
        orchestrators_active: u8,
        orchestrators_total: u8,
        utilities_active: u8,
        utilities_total: u8,
        current_view: &str,
    ) -> Self {
        Self {
            workers_active,
            workers_total,
            orchestrators_active,
            orchestrators_total,
            utilities_active,
            utilities_total,
            current_view: current_view.to_string(),
            mode_indicator: String::new(),
        }
    }

    /// Set the mode indicator (e.g., "[REFACTOR]")
    pub fn with_mode(mut self, mode: &str) -> Self {
        self.mode_indicator = mode.to_string();
        self
    }
}

impl Widget for HeaderBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Build status string
        let status = format!(
            "w[{}/{}] o[{}/{}] u[{}/{}]",
            self.workers_active,
            self.workers_total,
            self.orchestrators_active,
            self.orchestrators_total,
            self.utilities_active,
            self.utilities_total
        );

        // Render mode indicator and status on left
        let left_content = if self.mode_indicator.is_empty() {
            status
        } else {
            format!("{} {}", self.mode_indicator, status)
        };

        let style = if !self.mode_indicator.is_empty() {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Cyan)
        };

        buf.set_string(area.x, area.y, &left_content, style);

        // Render view name on right
        let view_x = area
            .right()
            .saturating_sub(self.current_view.len() as u16 + 1);
        buf.set_string(
            view_x,
            area.y,
            &self.current_view,
            Style::default().fg(Color::DarkGray),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_splits_area_correctly() {
        let area = Rect::new(0, 0, 80, 24);
        let layout = AppLayout::new(area);

        // Header is 1 line
        assert_eq!(layout.header.height, 1);
        assert_eq!(layout.header.y, 0);

        // Input is 2 lines at bottom
        assert_eq!(layout.input.height, 2);
        assert_eq!(layout.input.y, 22); // 24 - 2

        // Main takes the rest
        assert_eq!(layout.main.height, 21); // 24 - 1 - 2
        assert_eq!(layout.main.y, 1);
    }

    #[test]
    fn header_bar_default_values() {
        let header = HeaderBar::default();
        assert_eq!(header.workers_total, 6);
        assert_eq!(header.orchestrators_total, 2);
        assert_eq!(header.utilities_total, 4);
    }

    #[test]
    fn header_bar_with_mode() {
        let header = HeaderBar::default().with_mode("[REFACTOR]");
        assert_eq!(header.mode_indicator, "[REFACTOR]");
    }
}
