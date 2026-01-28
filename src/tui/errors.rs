//! Error display widget for TUI
//!
//! Shows recent errors with timestamps, color-coded by severity,
//! with recovery suggestions.

use crate::error::NexorError;
use chrono::{DateTime, Local};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};

/// A displayable error with metadata
#[derive(Debug, Clone)]
pub struct DisplayError {
    /// The actual error
    pub error: NexorError,
    /// When the error occurred
    pub timestamp: DateTime<Local>,
    /// Whether the user has dismissed this error
    pub dismissed: bool,
}

impl DisplayError {
    /// Create a new display error from a NexorError
    pub fn new(error: NexorError) -> Self {
        Self {
            error,
            timestamp: Local::now(),
            dismissed: false,
        }
    }
}

/// Error display state and widget
pub struct ErrorDisplay {
    /// All errors (including dismissed)
    pub errors: Vec<DisplayError>,
    /// Whether the error panel is expanded
    pub expanded: bool,
    /// Maximum number of errors to show when expanded
    pub max_visible: usize,
}

impl Default for ErrorDisplay {
    fn default() -> Self {
        Self {
            errors: Vec::new(),
            expanded: false,
            max_visible: 5,
        }
    }
}

impl ErrorDisplay {
    /// Create a new error display
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new error to the display
    pub fn push(&mut self, error: NexorError) {
        self.errors.push(DisplayError::new(error));
        // Auto-expand on new error
        self.expanded = true;
    }

    /// Dismiss all active errors
    pub fn dismiss_all(&mut self) {
        for e in &mut self.errors {
            e.dismissed = true;
        }
        self.expanded = false;
    }

    /// Toggle the expanded state
    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Get count of active (non-dismissed) errors
    pub fn active_count(&self) -> usize {
        self.errors.iter().filter(|e| !e.dismissed).count()
    }

    /// Get recent active errors
    pub fn recent_errors(&self) -> Vec<&DisplayError> {
        self.errors
            .iter()
            .filter(|e| !e.dismissed)
            .rev()
            .take(self.max_visible)
            .collect()
    }

    /// Clear all errors (including dismissed)
    pub fn clear(&mut self) {
        self.errors.clear();
        self.expanded = false;
    }

    /// Check if there are any active errors
    pub fn has_errors(&self) -> bool {
        self.active_count() > 0
    }

    /// Calculate required height for rendering
    pub fn required_height(&self) -> u16 {
        if !self.expanded {
            if self.has_errors() {
                1 // Collapsed indicator
            } else {
                0 // Nothing to show
            }
        } else {
            // Each error takes 1-2 lines, plus 2 for border
            let errors = self.recent_errors();
            let lines: usize = errors
                .iter()
                .map(|e| if e.error.suggestion().is_some() { 2 } else { 1 })
                .sum();
            (lines + 2).min(10) as u16
        }
    }

    /// Render the collapsed indicator
    fn render_collapsed(&self, area: Rect, buf: &mut Buffer) {
        let active = self.active_count();
        if active > 0 {
            let text = format!(" {} error(s) - press 'e' to expand ", active);
            let style = Style::default().fg(Color::Yellow);
            buf.set_string(area.x, area.y, &text, style);
        }
    }

    /// Render the expanded error list
    fn render_expanded(&self, area: Rect, buf: &mut Buffer) {
        let active = self.active_count();
        let errors = self.recent_errors();

        let items: Vec<ListItem> = errors
            .iter()
            .map(|de| {
                let time = de.timestamp.format("%H:%M:%S").to_string();
                let color = if de.error.is_recoverable() {
                    Color::Yellow
                } else {
                    Color::Red
                };

                let mut lines = vec![Line::from(vec![
                    Span::styled(format!("[{}] ", time), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{}", de.error), Style::default().fg(color)),
                ])];

                if let Some(suggestion) = de.error.suggestion() {
                    lines.push(Line::from(vec![Span::styled(
                        format!("  -> {}", suggestion),
                        Style::default().fg(Color::Cyan),
                    )]));
                }

                ListItem::new(lines)
            })
            .collect();

        let title = format!(" Errors ({}) - 'd' to dismiss, 'e' to collapse ", active);
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(title),
        );

        Widget::render(list, area, buf);
    }
}

impl Widget for ErrorDisplay {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.expanded {
            self.render_collapsed(area, buf);
        } else {
            self.render_expanded(area, buf);
        }
    }
}

/// Borrowed version for rendering without consuming
impl Widget for &ErrorDisplay {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.expanded {
            let active = self.active_count();
            if active > 0 {
                let text = format!(" {} error(s) - press 'e' to expand ", active);
                let style = Style::default().fg(Color::Yellow);
                buf.set_string(area.x, area.y, &text, style);
            }
        } else {
            let active = self.active_count();
            let errors = self.recent_errors();

            let items: Vec<ListItem> = errors
                .iter()
                .map(|de| {
                    let time = de.timestamp.format("%H:%M:%S").to_string();
                    let color = if de.error.is_recoverable() {
                        Color::Yellow
                    } else {
                        Color::Red
                    };

                    let mut lines = vec![Line::from(vec![
                        Span::styled(format!("[{}] ", time), Style::default().fg(Color::DarkGray)),
                        Span::styled(format!("{}", de.error), Style::default().fg(color)),
                    ])];

                    if let Some(suggestion) = de.error.suggestion() {
                        lines.push(Line::from(vec![Span::styled(
                            format!("  -> {}", suggestion),
                            Style::default().fg(Color::Cyan),
                        )]));
                    }

                    ListItem::new(lines)
                })
                .collect();

            let title = format!(" Errors ({}) - 'd' to dismiss, 'e' to collapse ", active);
            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red))
                    .title(title),
            );

            Widget::render(list, area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_default() {
        let display = ErrorDisplay::default();
        assert!(display.errors.is_empty());
        assert!(!display.expanded);
        assert_eq!(display.max_visible, 5);
    }

    #[test]
    fn push_error_auto_expands() {
        let mut display = ErrorDisplay::new();
        assert!(!display.expanded);

        display.push(NexorError::internal("test"));
        assert!(display.expanded);
        assert_eq!(display.errors.len(), 1);
    }

    #[test]
    fn active_count() {
        let mut display = ErrorDisplay::new();
        display.push(NexorError::internal("error 1"));
        display.push(NexorError::internal("error 2"));
        assert_eq!(display.active_count(), 2);

        display.errors[0].dismissed = true;
        assert_eq!(display.active_count(), 1);
    }

    #[test]
    fn dismiss_all() {
        let mut display = ErrorDisplay::new();
        display.push(NexorError::internal("error 1"));
        display.push(NexorError::internal("error 2"));
        assert!(display.expanded);

        display.dismiss_all();
        assert!(!display.expanded);
        assert_eq!(display.active_count(), 0);
        assert_eq!(display.errors.len(), 2); // Still there, just dismissed
    }

    #[test]
    fn toggle() {
        let mut display = ErrorDisplay::new();
        assert!(!display.expanded);

        display.toggle();
        assert!(display.expanded);

        display.toggle();
        assert!(!display.expanded);
    }

    #[test]
    fn recent_errors_limit() {
        let mut display = ErrorDisplay::new();
        display.max_visible = 2;

        display.push(NexorError::internal("error 1"));
        display.push(NexorError::internal("error 2"));
        display.push(NexorError::internal("error 3"));

        let recent = display.recent_errors();
        assert_eq!(recent.len(), 2);
        // Should be most recent first
        assert!(recent[0].error.to_string().contains("error 3"));
        assert!(recent[1].error.to_string().contains("error 2"));
    }

    #[test]
    fn recent_errors_excludes_dismissed() {
        let mut display = ErrorDisplay::new();
        display.push(NexorError::internal("error 1"));
        display.push(NexorError::internal("error 2"));
        display.errors[0].dismissed = true;

        let recent = display.recent_errors();
        assert_eq!(recent.len(), 1);
        assert!(recent[0].error.to_string().contains("error 2"));
    }

    #[test]
    fn clear_removes_all() {
        let mut display = ErrorDisplay::new();
        display.push(NexorError::internal("error 1"));
        display.push(NexorError::internal("error 2"));

        display.clear();
        assert!(display.errors.is_empty());
        assert!(!display.expanded);
    }

    #[test]
    fn has_errors() {
        let mut display = ErrorDisplay::new();
        assert!(!display.has_errors());

        display.push(NexorError::internal("test"));
        assert!(display.has_errors());

        display.dismiss_all();
        assert!(!display.has_errors());
    }

    #[test]
    fn required_height_no_errors() {
        let display = ErrorDisplay::new();
        assert_eq!(display.required_height(), 0);
    }

    #[test]
    fn required_height_collapsed() {
        let mut display = ErrorDisplay::new();
        display.push(NexorError::internal("test"));
        display.expanded = false;
        assert_eq!(display.required_height(), 1);
    }

    #[test]
    fn required_height_expanded() {
        let mut display = ErrorDisplay::new();
        display.push(NexorError::internal("test")); // 1 line
        display.push(NexorError::api_key_missing("anthropic")); // 2 lines (with suggestion)
        display.expanded = true;
        // 3 lines + 2 for border = 5
        assert!(display.required_height() >= 4);
    }

    #[test]
    fn display_error_timestamp() {
        let error = DisplayError::new(NexorError::internal("test"));
        let now = Local::now();
        // Should be within a second
        let diff = now.signed_duration_since(error.timestamp);
        assert!(diff.num_seconds() < 1);
    }

    #[test]
    fn display_error_not_dismissed_by_default() {
        let error = DisplayError::new(NexorError::internal("test"));
        assert!(!error.dismissed);
    }
}
