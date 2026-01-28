//! Home screen view with branding and welcome message.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// ASCII art logo for the home screen.
const LOGO: &str = r#"
  ███╗   ██╗███████╗██╗  ██╗ ██████╗ ██████╗
  ████╗  ██║██╔════╝╚██╗██╔╝██╔═══██╗██╔══██╗
  ██╔██╗ ██║█████╗   ╚███╔╝ ██║   ██║██████╔╝
  ██║╚██╗██║██╔══╝   ██╔██╗ ██║   ██║██╔══██╗
  ██║ ╚████║███████╗██╔╝ ╚██╗╚██████╔╝██║  ██║
  ╚═╝  ╚═══╝╚══════╝╚═╝   ╚═╝ ╚═════╝ ╚═╝  ╚═╝
"#;

/// Subtitle below the logo.
const SUBTITLE: &str = "AI Agent Orchestration TUI for GitHub Workflows";

/// Home screen view widget.
pub struct HomeView {
    /// Optional status message to display.
    pub status_message: Option<String>,
    /// Whether to show the quick commands.
    pub show_commands: bool,
}

impl Default for HomeView {
    fn default() -> Self {
        Self {
            status_message: None,
            show_commands: true,
        }
    }
}

impl HomeView {
    /// Create a new home view with a status message.
    pub fn with_message(mut self, message: &str) -> Self {
        self.status_message = Some(message.to_string());
        self
    }

    /// Hide the quick commands list.
    pub fn hide_commands(mut self) -> Self {
        self.show_commands = false;
        self
    }
}

impl Widget for HomeView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 {
            return;
        }

        // Collect all content lines
        let mut lines: Vec<(&str, Style)> = Vec::new();

        // Add logo lines
        let logo_style = Style::default().fg(Color::Cyan);
        for line in LOGO.lines() {
            lines.push((line, logo_style));
        }

        // Add subtitle
        lines.push(("", Style::default())); // blank line
        let subtitle_style = Style::default().fg(Color::White);
        lines.push((SUBTITLE, subtitle_style));

        // Calculate total content height
        let commands_height = if self.show_commands { 8 } else { 0 };
        let message_height = if self.status_message.is_some() { 2 } else { 0 };
        let content_height = lines.len() as u16 + commands_height + message_height;

        // Calculate vertical centering
        let start_y = area.y + area.height.saturating_sub(content_height) / 2;

        // Render logo and subtitle centered
        for (i, (line, style)) in lines.iter().enumerate() {
            let y = start_y + i as u16;
            if y >= area.bottom() {
                break;
            }
            let x = area.x + area.width.saturating_sub(line.len() as u16) / 2;
            buf.set_string(x, y, line, *style);
        }

        // Render status message if any
        let mut current_y = start_y + lines.len() as u16;
        if let Some(ref msg) = self.status_message {
            current_y += 1;
            if current_y < area.bottom() {
                let x = area.x + area.width.saturating_sub(msg.len() as u16) / 2;
                buf.set_string(x, current_y, msg, Style::default().fg(Color::Yellow));
            }
            current_y += 1;
        }

        // Render quick commands
        if self.show_commands && current_y + 6 < area.bottom() {
            current_y += 1;

            let commands = [
                ("Commands:", Style::default().fg(Color::White)),
                ("  /main     - Chat with orchestrator", Style::default().fg(Color::DarkGray)),
                ("  /feed     - View agent activity", Style::default().fg(Color::DarkGray)),
                ("  /logs     - View technical logs", Style::default().fg(Color::DarkGray)),
                ("  /refactor - Enter refactor mode", Style::default().fg(Color::DarkGray)),
                ("  /help     - Show all commands", Style::default().fg(Color::DarkGray)),
                ("  /quit     - Exit nexor", Style::default().fg(Color::DarkGray)),
            ];

            for (cmd, style) in commands {
                if current_y >= area.bottom() {
                    break;
                }
                let x = area.x + area.width.saturating_sub(cmd.len() as u16) / 2;
                buf.set_string(x, current_y, cmd, style);
                current_y += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_view_default() {
        let view = HomeView::default();
        assert!(view.status_message.is_none());
        assert!(view.show_commands);
    }

    #[test]
    fn home_view_with_message() {
        let view = HomeView::default().with_message("Loading...");
        assert_eq!(view.status_message, Some("Loading...".to_string()));
    }

    #[test]
    fn home_view_hide_commands() {
        let view = HomeView::default().hide_commands();
        assert!(!view.show_commands);
    }

    #[test]
    fn logo_is_not_empty() {
        assert!(!LOGO.is_empty());
        assert!(LOGO.lines().count() > 5);
    }
}
