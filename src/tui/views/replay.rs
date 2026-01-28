//! Replay view for visualizing LLM call timelines

use crate::observability::LlmCall;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::Widget,
};

/// View for replaying and inspecting LLM calls
pub struct ReplayView {
    /// All LLM calls in the timeline
    pub calls: Vec<LlmCall>,
    /// Currently selected call index
    pub selected_index: usize,
    /// Scroll offset for the response view
    pub scroll_offset: usize,
    /// Whether to show the full prompt (vs summary)
    pub show_full_prompt: bool,
    /// Total cost for all calls
    pub total_cost: f64,
    /// Total tokens for all calls
    pub total_tokens: u32,
}

impl Default for ReplayView {
    fn default() -> Self {
        Self {
            calls: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            show_full_prompt: false,
            total_cost: 0.0,
            total_tokens: 0,
        }
    }
}

impl ReplayView {
    /// Create a new replay view
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the calls to display
    pub fn with_calls(mut self, calls: Vec<LlmCall>) -> Self {
        self.total_cost = calls.iter().map(|c| c.cost_usd).sum();
        self.total_tokens = calls.iter().map(|c: &LlmCall| c.total_tokens()).sum();
        self.calls = calls;
        self
    }

    /// Select the next call
    pub fn select_next(&mut self) {
        if self.selected_index < self.calls.len().saturating_sub(1) {
            self.selected_index += 1;
            self.scroll_offset = 0;
        }
    }

    /// Select the previous call
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.scroll_offset = 0;
        }
    }

    /// Toggle showing full prompt
    pub fn toggle_full_prompt(&mut self) {
        self.show_full_prompt = !self.show_full_prompt;
    }

    /// Scroll down in the response
    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    /// Scroll up in the response
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Get the currently selected call
    pub fn current_call(&self) -> Option<&LlmCall> {
        self.calls.get(self.selected_index)
    }

    /// Check if the view is empty
    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }
}

impl Widget for &ReplayView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.calls.is_empty() {
            buf.set_string(
                area.x + 2,
                area.y + 2,
                "No LLM calls found for this task.",
                Style::default().fg(Color::DarkGray),
            );
            buf.set_string(
                area.x + 2,
                area.y + 4,
                "Use /replay <task_id> to view a task's timeline.",
                Style::default().fg(Color::DarkGray),
            );
            return;
        }

        // Split into timeline (left) and detail (right)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(area);

        self.render_timeline(chunks[0], buf);
        self.render_detail(chunks[1], buf);
    }
}

impl ReplayView {
    fn render_timeline(&self, area: Rect, buf: &mut Buffer) {
        // Header
        let header = format!(
            "Timeline ({} calls, ${:.4})",
            self.calls.len(),
            self.total_cost
        );
        buf.set_string(
            area.x,
            area.y,
            &header,
            Style::default().add_modifier(Modifier::BOLD),
        );

        // Instructions
        buf.set_string(
            area.x,
            area.y + 1,
            "j/k navigate, Enter expand, q quit",
            Style::default().fg(Color::DarkGray),
        );

        let mut y = area.y + 3;

        for (i, call) in self.calls.iter().enumerate() {
            if y >= area.bottom() - 1 {
                break;
            }

            let style = if i == self.selected_index {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Format: HH:MM:SS model $0.0000
            let time = call.timestamp.format("%H:%M:%S").to_string();
            let model_short = call
                .model
                .split('/')
                .last()
                .unwrap_or(&call.model)
                .chars()
                .take(15)
                .collect::<String>();
            let line = format!("{} {:15} ${:.4}", time, model_short, call.cost_usd);

            // Truncate to fit
            let max_width = (area.width as usize).saturating_sub(2);
            let display_line: String = line.chars().take(max_width).collect();

            buf.set_string(area.x, y, &display_line, style);
            y += 1;
        }
    }

    fn render_detail(&self, area: Rect, buf: &mut Buffer) {
        let Some(call) = self.calls.get(self.selected_index) else {
            return;
        };

        let mut y = area.y;

        // Header with stats
        let header = format!(
            "Call {} of {} | {} | {} in / {} out tokens | ${:.4}",
            self.selected_index + 1,
            self.calls.len(),
            call.model
                .split('/')
                .last()
                .unwrap_or(&call.model),
            call.input_tokens,
            call.output_tokens,
            call.cost_usd
        );
        buf.set_string(
            area.x,
            y,
            &header,
            Style::default().add_modifier(Modifier::BOLD),
        );
        y += 1;

        let latency_info = format!("Latency: {}ms", call.latency_ms);
        buf.set_string(area.x, y, &latency_info, Style::default().fg(Color::DarkGray));
        y += 2;

        // System prompt section
        buf.set_string(
            area.x,
            y,
            "SYSTEM PROMPT:",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        );
        y += 1;

        let system_preview = if self.show_full_prompt {
            call.prompt.system.clone()
        } else {
            let preview: String = call.prompt.system.chars().take(200).collect();
            if call.prompt.system.len() > 200 {
                format!("{}... (press Enter for full)", preview)
            } else {
                preview
            }
        };

        let max_width = (area.width as usize).saturating_sub(2);
        for line in system_preview.lines().take(if self.show_full_prompt { 20 } else { 4 }) {
            if y >= area.bottom() - 10 {
                break;
            }
            let display = line.chars().take(max_width).collect::<String>();
            buf.set_string(area.x + 1, y, &display, Style::default());
            y += 1;
        }
        y += 1;

        // User messages
        if !call.prompt.messages.is_empty() {
            buf.set_string(
                area.x,
                y,
                "MESSAGES:",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            );
            y += 1;

            for msg in call.prompt.messages.iter().take(3) {
                if y >= area.bottom() - 6 {
                    break;
                }
                let role_color = if msg.role == "user" {
                    Color::Blue
                } else {
                    Color::Green
                };
                let preview: String = msg.content.chars().take(80).collect();
                let line = format!("[{}] {}", msg.role, preview);
                let display: String = line.chars().take(max_width).collect();
                buf.set_string(area.x + 1, y, &display, Style::default().fg(role_color));
                y += 1;
            }
            y += 1;
        }

        // Response section
        buf.set_string(
            area.x,
            y,
            "RESPONSE:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        );
        y += 1;

        let response_lines: Vec<&str> = call.response.lines().collect();
        let visible_lines = (area.bottom() - y) as usize;
        let start = self.scroll_offset.min(response_lines.len().saturating_sub(visible_lines));

        for line in response_lines.iter().skip(start).take(visible_lines) {
            if y >= area.bottom() {
                break;
            }
            let display = line.chars().take(max_width).collect::<String>();
            buf.set_string(area.x + 1, y, &display, Style::default());
            y += 1;
        }

        // Scroll indicator
        if response_lines.len() > visible_lines {
            let indicator = format!(
                "[{}/{}]",
                start + 1,
                response_lines.len().saturating_sub(visible_lines) + 1
            );
            buf.set_string(
                area.right().saturating_sub(indicator.len() as u16 + 1),
                area.bottom() - 1,
                &indicator,
                Style::default().fg(Color::DarkGray),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::LlmPrompt;

    fn mock_call(model: &str, cost: f64, tokens: u32) -> LlmCall {
        LlmCall::new(model, LlmPrompt::new("System prompt here"), "Response text here")
            .with_cost(cost)
            .with_tokens(tokens / 2, tokens / 2)
            .with_latency(500)
    }

    #[test]
    fn replay_view_default() {
        let view = ReplayView::default();
        assert!(view.is_empty());
        assert_eq!(view.selected_index, 0);
        assert!(!view.show_full_prompt);
    }

    #[test]
    fn replay_view_with_calls() {
        let calls = vec![
            mock_call("model-a", 0.01, 100),
            mock_call("model-b", 0.02, 200),
        ];
        let view = ReplayView::new().with_calls(calls);

        assert!(!view.is_empty());
        assert_eq!(view.calls.len(), 2);
        assert!((view.total_cost - 0.03).abs() < f64::EPSILON);
        assert_eq!(view.total_tokens, 300);
    }

    #[test]
    fn replay_view_navigation() {
        let calls = vec![
            mock_call("model-a", 0.01, 100),
            mock_call("model-b", 0.02, 200),
            mock_call("model-c", 0.03, 300),
        ];
        let mut view = ReplayView::new().with_calls(calls);

        assert_eq!(view.selected_index, 0);

        view.select_next();
        assert_eq!(view.selected_index, 1);

        view.select_next();
        assert_eq!(view.selected_index, 2);

        // Can't go past end
        view.select_next();
        assert_eq!(view.selected_index, 2);

        view.select_prev();
        assert_eq!(view.selected_index, 1);

        view.select_prev();
        assert_eq!(view.selected_index, 0);

        // Can't go before start
        view.select_prev();
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn replay_view_toggle_prompt() {
        let mut view = ReplayView::new();
        assert!(!view.show_full_prompt);

        view.toggle_full_prompt();
        assert!(view.show_full_prompt);

        view.toggle_full_prompt();
        assert!(!view.show_full_prompt);
    }

    #[test]
    fn replay_view_scroll() {
        let mut view = ReplayView::new();
        assert_eq!(view.scroll_offset, 0);

        view.scroll_down();
        assert_eq!(view.scroll_offset, 1);

        view.scroll_down();
        assert_eq!(view.scroll_offset, 2);

        view.scroll_up();
        assert_eq!(view.scroll_offset, 1);

        view.scroll_up();
        assert_eq!(view.scroll_offset, 0);

        // Can't go below 0
        view.scroll_up();
        assert_eq!(view.scroll_offset, 0);
    }

    #[test]
    fn replay_view_current_call() {
        let calls = vec![
            mock_call("model-a", 0.01, 100),
            mock_call("model-b", 0.02, 200),
        ];
        let mut view = ReplayView::new().with_calls(calls);

        assert_eq!(view.current_call().unwrap().model, "model-a");

        view.select_next();
        assert_eq!(view.current_call().unwrap().model, "model-b");
    }

    #[test]
    fn replay_view_current_call_empty() {
        let view = ReplayView::new();
        assert!(view.current_call().is_none());
    }

    #[test]
    fn replay_view_navigation_resets_scroll() {
        let calls = vec![
            mock_call("model-a", 0.01, 100),
            mock_call("model-b", 0.02, 200),
        ];
        let mut view = ReplayView::new().with_calls(calls);

        view.scroll_down();
        view.scroll_down();
        assert_eq!(view.scroll_offset, 2);

        view.select_next();
        assert_eq!(view.scroll_offset, 0);
    }
}
