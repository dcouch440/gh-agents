//! Chat view for orchestrator conversation interface.

use chrono::{DateTime, Utc};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget, Wrap},
};

/// Who sent a chat message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSender {
    /// Message from the user.
    User,
    /// Message from the orchestrator agent.
    Orchestrator,
}

/// A single chat message.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    /// Who sent this message.
    pub sender: MessageSender,
    /// The message content.
    pub content: String,
    /// When the message was sent.
    pub timestamp: DateTime<Utc>,
    /// True while response is being received (streaming).
    pub is_streaming: bool,
}

impl ChatMessage {
    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            sender: MessageSender::User,
            content: content.into(),
            timestamp: Utc::now(),
            is_streaming: false,
        }
    }

    /// Create an orchestrator message.
    pub fn orchestrator(content: impl Into<String>) -> Self {
        Self {
            sender: MessageSender::Orchestrator,
            content: content.into(),
            timestamp: Utc::now(),
            is_streaming: false,
        }
    }

    /// Create a streaming orchestrator message (in-progress).
    pub fn orchestrator_streaming() -> Self {
        Self {
            sender: MessageSender::Orchestrator,
            content: String::new(),
            timestamp: Utc::now(),
            is_streaming: true,
        }
    }

    /// Append content to a streaming message.
    pub fn append(&mut self, text: &str) {
        self.content.push_str(text);
    }

    /// Mark the message as complete (stop streaming).
    pub fn complete(&mut self) {
        self.is_streaming = false;
    }
}

/// Chat view widget showing conversation with the orchestrator.
#[derive(Debug, Clone, Default)]
pub struct ChatView {
    /// Messages in the conversation.
    pub messages: Vec<ChatMessage>,
    /// Current scroll offset (0 = top).
    pub scroll_offset: usize,
}

impl ChatView {
    /// Create a new empty chat view.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a message to the chat.
    pub fn push(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    /// Get the number of messages.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if the chat is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Scroll to show the bottom of the chat.
    pub fn scroll_to_bottom(&mut self, visible_lines: usize) {
        // This is a simplification; proper implementation would calculate
        // actual rendered line count including wrapped lines
        let total_lines = self.messages.len() * 2; // rough estimate
        if total_lines > visible_lines {
            self.scroll_offset = total_lines - visible_lines;
        } else {
            self.scroll_offset = 0;
        }
    }
}

impl Widget for ChatView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Handle empty state
        if self.messages.is_empty() {
            let hint = "Type a message to start chatting with the orchestrator...";
            let x = area.x + 1;
            let y = area.y + 1;
            if y < area.bottom() {
                buf.set_string(x, y, hint, Style::default().fg(Color::DarkGray));
            }
            return;
        }

        let mut lines: Vec<Line> = Vec::new();

        for msg in &self.messages {
            // Add blank line between messages
            if !lines.is_empty() {
                lines.push(Line::from(""));
            }

            let (prefix, style) = match msg.sender {
                MessageSender::User => ("You: ", Style::default().fg(Color::Green)),
                MessageSender::Orchestrator => ("Orchestrator: ", Style::default().fg(Color::Cyan)),
            };

            // Build content with streaming indicator if applicable
            let content = if msg.is_streaming && msg.content.is_empty() {
                "▋".to_string() // Cursor for empty streaming message
            } else if msg.is_streaming {
                format!("{}▋", msg.content) // Content + cursor
            } else {
                msg.content.clone()
            };

            // First line with sender prefix
            lines.push(Line::from(vec![
                Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
                Span::raw(content),
            ]));
        }

        let text = Text::from(lines);
        let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
        Widget::render(paragraph, area, buf);
    }
}

/// Response from the orchestrator agent.
#[derive(Debug, Clone)]
pub enum OrchestratorResponse {
    /// Response started streaming.
    Start,
    /// Partial response content (streaming chunk).
    Chunk(String),
    /// Final complete response.
    Complete(String),
    /// Error occurred.
    Error(String),
}

/// Message sent from user to orchestrator.
#[derive(Debug, Clone)]
pub struct UserMessage {
    /// The message content.
    pub content: String,
}

impl UserMessage {
    /// Create a new user message.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_message_user() {
        let msg = ChatMessage::user("Hello!");
        assert_eq!(msg.sender, MessageSender::User);
        assert_eq!(msg.content, "Hello!");
        assert!(!msg.is_streaming);
    }

    #[test]
    fn chat_message_orchestrator() {
        let msg = ChatMessage::orchestrator("Hi there");
        assert_eq!(msg.sender, MessageSender::Orchestrator);
        assert_eq!(msg.content, "Hi there");
        assert!(!msg.is_streaming);
    }

    #[test]
    fn chat_message_orchestrator_streaming() {
        let msg = ChatMessage::orchestrator_streaming();
        assert_eq!(msg.sender, MessageSender::Orchestrator);
        assert!(msg.content.is_empty());
        assert!(msg.is_streaming);
    }

    #[test]
    fn chat_message_append() {
        let mut msg = ChatMessage::orchestrator_streaming();
        msg.append("Hello");
        msg.append(" world");
        assert_eq!(msg.content, "Hello world");
        assert!(msg.is_streaming);
    }

    #[test]
    fn chat_message_complete() {
        let mut msg = ChatMessage::orchestrator_streaming();
        msg.append("Response");
        msg.complete();
        assert!(!msg.is_streaming);
    }

    #[test]
    fn chat_view_default_is_empty() {
        let view = ChatView::default();
        assert!(view.is_empty());
        assert_eq!(view.len(), 0);
    }

    #[test]
    fn chat_view_push() {
        let mut view = ChatView::new();
        view.push(ChatMessage::user("Hello"));
        view.push(ChatMessage::orchestrator("Hi"));
        assert_eq!(view.len(), 2);
    }

    #[test]
    fn user_message_new() {
        let msg = UserMessage::new("Test message");
        assert_eq!(msg.content, "Test message");
    }

    #[test]
    fn orchestrator_response_variants() {
        let _start = OrchestratorResponse::Start;
        let _chunk = OrchestratorResponse::Chunk("text".to_string());
        let _complete = OrchestratorResponse::Complete("final".to_string());
        let _error = OrchestratorResponse::Error("oops".to_string());
    }
}
