//! Logs view showing technical application logs.

use chrono::{DateTime, Local};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// Log level matching tracing levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl LogLevel {
    /// Get the string representation of the log level.
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }

    /// Get the color for this log level.
    pub fn color(&self) -> Color {
        match self {
            LogLevel::Error => Color::Red,
            LogLevel::Warn => Color::Yellow,
            LogLevel::Info => Color::Green,
            LogLevel::Debug => Color::Blue,
            LogLevel::Trace => Color::DarkGray,
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

/// A single log entry.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Log level.
    pub level: LogLevel,
    /// Module/target that generated the log.
    pub target: String,
    /// The log message.
    pub message: String,
    /// When the log was created.
    pub timestamp: DateTime<Local>,
}

impl LogEntry {
    /// Create a new log entry with the current timestamp.
    pub fn new(level: LogLevel, target: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level,
            target: target.into(),
            message: message.into(),
            timestamp: Local::now(),
        }
    }

    /// Create an error log entry.
    pub fn error(target: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(LogLevel::Error, target, message)
    }

    /// Create a warning log entry.
    pub fn warn(target: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(LogLevel::Warn, target, message)
    }

    /// Create an info log entry.
    pub fn info(target: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(LogLevel::Info, target, message)
    }

    /// Create a debug log entry.
    pub fn debug(target: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(LogLevel::Debug, target, message)
    }
}

/// Logs view widget showing technical application logs.
#[derive(Debug, Clone, Default)]
pub struct LogsView {
    /// All log entries.
    pub entries: Vec<LogEntry>,
    /// Current scroll offset (0 = top).
    pub scroll_offset: usize,
    /// Minimum log level to display (filters out lower priority).
    pub min_level: LogLevel,
}

impl LogsView {
    /// Create a new empty logs view.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a log entry.
    pub fn push(&mut self, entry: LogEntry) {
        self.entries.push(entry);
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if there are no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get entries filtered by the minimum level.
    pub fn filtered_entries(&self) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.level <= self.min_level)
            .collect()
    }

    /// Set the minimum log level filter.
    pub fn set_min_level(&mut self, level: LogLevel) {
        self.min_level = level;
    }

    /// Scroll to the bottom of the logs.
    pub fn scroll_to_bottom(&mut self, visible_height: usize) {
        let filtered_count = self.filtered_entries().len();
        if filtered_count > visible_height {
            self.scroll_offset = filtered_count - visible_height;
        } else {
            self.scroll_offset = 0;
        }
    }

    /// Scroll up.
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Scroll down.
    pub fn scroll_down(&mut self, amount: usize, visible_height: usize) {
        let filtered_count = self.filtered_entries().len();
        let max_offset = filtered_count.saturating_sub(visible_height);
        self.scroll_offset = (self.scroll_offset + amount).min(max_offset);
    }
}

impl Widget for LogsView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Handle empty state
        if self.entries.is_empty() {
            let msg = "No logs yet...";
            let x = area.x + 1;
            let y = area.y + 1;
            if y < area.bottom() {
                buf.set_string(x, y, msg, Style::default().fg(Color::DarkGray));
            }
            return;
        }

        let visible_height = area.height as usize;
        let filtered: Vec<&LogEntry> = self.filtered_entries();

        // Apply scroll offset
        let display_entries: Vec<&LogEntry> = filtered
            .iter()
            .skip(self.scroll_offset)
            .take(visible_height)
            .copied()
            .collect();

        for (i, entry) in display_entries.iter().enumerate() {
            let y = area.y + i as u16;
            if y >= area.bottom() {
                break;
            }

            let mut x = area.x;

            // Timestamp: HH:MM:SS
            let time = entry.timestamp.format("%H:%M:%S").to_string();
            buf.set_string(x, y, &time, Style::default().fg(Color::DarkGray));
            x += time.len() as u16 + 1;

            // Level (padded to 5 chars)
            let level_str = format!("{:5}", entry.level.as_str());
            buf.set_string(x, y, &level_str, Style::default().fg(entry.level.color()));
            x += level_str.len() as u16 + 1;

            // Target (module) - truncate if needed
            let max_target_len: usize = 20;
            let target = if entry.target.len() > max_target_len {
                format!(
                    "...{}",
                    &entry.target[entry.target.len() - max_target_len + 3..]
                )
            } else {
                format!("{:width$}", entry.target, width = max_target_len)
            };
            buf.set_string(x, y, &target, Style::default().fg(Color::Cyan));
            x += max_target_len as u16 + 1;

            // Message - fill remaining width
            let remaining = area.right().saturating_sub(x) as usize;
            if remaining > 0 {
                let message = if entry.message.len() > remaining {
                    format!("{}...", &entry.message[..remaining.saturating_sub(3)])
                } else {
                    entry.message.clone()
                };
                buf.set_string(x, y, &message, Style::default());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_level_ordering() {
        // Error is highest priority (lowest number)
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }

    #[test]
    fn log_level_as_str() {
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Trace.as_str(), "TRACE");
    }

    #[test]
    fn log_entry_new() {
        let entry = LogEntry::new(LogLevel::Info, "nexor::tui", "Application started");
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.target, "nexor::tui");
        assert_eq!(entry.message, "Application started");
    }

    #[test]
    fn log_entry_helpers() {
        let error = LogEntry::error("test", "error message");
        assert_eq!(error.level, LogLevel::Error);

        let warn = LogEntry::warn("test", "warning message");
        assert_eq!(warn.level, LogLevel::Warn);

        let info = LogEntry::info("test", "info message");
        assert_eq!(info.level, LogLevel::Info);

        let debug = LogEntry::debug("test", "debug message");
        assert_eq!(debug.level, LogLevel::Debug);
    }

    #[test]
    fn logs_view_default_is_empty() {
        let view = LogsView::default();
        assert!(view.is_empty());
        assert_eq!(view.len(), 0);
        assert_eq!(view.min_level, LogLevel::Info);
    }

    #[test]
    fn logs_view_push() {
        let mut view = LogsView::new();
        view.push(LogEntry::info("test", "message"));
        assert_eq!(view.len(), 1);
    }

    #[test]
    fn logs_view_filtered_entries() {
        let mut view = LogsView::new();
        view.push(LogEntry::error("test", "error"));
        view.push(LogEntry::warn("test", "warn"));
        view.push(LogEntry::info("test", "info"));
        view.push(LogEntry::debug("test", "debug"));

        // Default min_level is Info, so should see Error, Warn, Info (3 entries)
        assert_eq!(view.filtered_entries().len(), 3);

        // Set to Error only
        view.set_min_level(LogLevel::Error);
        assert_eq!(view.filtered_entries().len(), 1);

        // Set to Debug to see all except Trace
        view.set_min_level(LogLevel::Debug);
        assert_eq!(view.filtered_entries().len(), 4);
    }

    #[test]
    fn logs_view_scroll() {
        let mut view = LogsView::new();
        for i in 0..20 {
            view.push(LogEntry::info("test", format!("Message {}", i)));
        }

        // Scroll down
        view.scroll_down(5, 10);
        assert_eq!(view.scroll_offset, 5);

        // Scroll up
        view.scroll_up(3);
        assert_eq!(view.scroll_offset, 2);

        // Scroll to bottom
        view.scroll_to_bottom(10);
        assert_eq!(view.scroll_offset, 10); // 20 entries - 10 visible = 10
    }
}
