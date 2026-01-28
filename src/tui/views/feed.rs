//! Feed view showing real-time agent activity.

use chrono::{DateTime, Utc};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Widget},
};

/// Type of feed item determining display style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedItemType {
    /// Regular agent status report.
    AgentReport,
    /// Task has been started by an agent.
    TaskStarted,
    /// Task completed successfully.
    TaskCompleted,
    /// Major milestone achieved.
    Milestone,
    /// Error or failure occurred.
    Error,
    /// System notification.
    SystemNotice,
}

impl FeedItemType {
    /// Get the icon and style for this item type.
    fn icon_and_style(&self) -> (&'static str, Style) {
        match self {
            FeedItemType::AgentReport => ("●", Style::default().fg(Color::White)),
            FeedItemType::TaskStarted => ("▶", Style::default().fg(Color::Blue)),
            FeedItemType::TaskCompleted => ("✓", Style::default().fg(Color::Green)),
            FeedItemType::Milestone => (
                "★",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            FeedItemType::Error => ("✗", Style::default().fg(Color::Red)),
            FeedItemType::SystemNotice => ("ℹ", Style::default().fg(Color::Cyan)),
        }
    }
}

/// A single item in the activity feed.
#[derive(Debug, Clone)]
pub struct FeedItem {
    /// Name of the agent or system component.
    pub agent_name: String,
    /// Content/message of the feed item.
    pub content: String,
    /// Type of feed item.
    pub item_type: FeedItemType,
    /// When this item was created.
    pub timestamp: DateTime<Utc>,
}

impl FeedItem {
    /// Create a new feed item with the current timestamp.
    pub fn new(
        agent_name: impl Into<String>,
        content: impl Into<String>,
        item_type: FeedItemType,
    ) -> Self {
        Self {
            agent_name: agent_name.into(),
            content: content.into(),
            item_type,
            timestamp: Utc::now(),
        }
    }

    /// Create a system notice.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new("System", content, FeedItemType::SystemNotice)
    }

    /// Create an error item.
    pub fn error(agent_name: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(agent_name, content, FeedItemType::Error)
    }

    /// Create a task started item.
    pub fn task_started(agent_name: impl Into<String>, task: impl Into<String>) -> Self {
        Self::new(
            agent_name,
            format!("Started: {}", task.into()),
            FeedItemType::TaskStarted,
        )
    }

    /// Create a task completed item.
    pub fn task_completed(agent_name: impl Into<String>, task: impl Into<String>) -> Self {
        Self::new(
            agent_name,
            format!("Completed: {}", task.into()),
            FeedItemType::TaskCompleted,
        )
    }
}

/// Feed view widget showing scrollable agent activity.
#[derive(Debug, Clone, Default)]
pub struct FeedView {
    /// Items in the feed.
    pub items: Vec<FeedItem>,
    /// Current scroll offset (0 = top).
    pub scroll_offset: usize,
}

impl FeedView {
    /// Create a new empty feed view.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an item to the feed.
    pub fn push(&mut self, item: FeedItem) {
        self.items.push(item);
    }

    /// Calculate and set scroll to show the bottom of the feed.
    pub fn scroll_to_bottom(&mut self, visible_height: usize) {
        if self.items.len() > visible_height {
            self.scroll_offset = self.items.len() - visible_height;
        } else {
            self.scroll_offset = 0;
        }
    }

    /// Scroll up by the given amount.
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Scroll down by the given amount, respecting max scroll.
    pub fn scroll_down(&mut self, amount: usize, visible_height: usize) {
        let max_offset = self.items.len().saturating_sub(visible_height);
        self.scroll_offset = (self.scroll_offset + amount).min(max_offset);
    }

    /// Get the number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the feed is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Widget for FeedView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Handle empty state
        if self.items.is_empty() {
            let empty_msg = "No agent activity yet. Start a task to see updates here.";
            let x = area.x + 2;
            let y = area.y + area.height / 2;
            buf.set_string(x, y, empty_msg, Style::default().fg(Color::DarkGray));
            return;
        }

        // Build list items with scroll offset applied
        let visible_height = area.height as usize;
        let items: Vec<ListItem> = self
            .items
            .iter()
            .skip(self.scroll_offset)
            .take(visible_height)
            .map(|item| {
                let (icon, style) = item.item_type.icon_and_style();

                let line = Line::from(vec![
                    Span::styled(format!("{} ", icon), style),
                    Span::styled(
                        format!("{}: ", item.agent_name),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(&item.content),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items);
        Widget::render(list, area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_item_new() {
        let item = FeedItem::new("Worker-1", "Processing task", FeedItemType::AgentReport);
        assert_eq!(item.agent_name, "Worker-1");
        assert_eq!(item.content, "Processing task");
        assert_eq!(item.item_type, FeedItemType::AgentReport);
    }

    #[test]
    fn feed_item_system() {
        let item = FeedItem::system("Started");
        assert_eq!(item.agent_name, "System");
        assert_eq!(item.item_type, FeedItemType::SystemNotice);
    }

    #[test]
    fn feed_item_task_started() {
        let item = FeedItem::task_started("Worker-1", "Implement login");
        assert!(item.content.contains("Started:"));
        assert_eq!(item.item_type, FeedItemType::TaskStarted);
    }

    #[test]
    fn feed_item_task_completed() {
        let item = FeedItem::task_completed("Worker-1", "Implement login");
        assert!(item.content.contains("Completed:"));
        assert_eq!(item.item_type, FeedItemType::TaskCompleted);
    }

    #[test]
    fn feed_view_default_is_empty() {
        let feed = FeedView::default();
        assert!(feed.is_empty());
        assert_eq!(feed.len(), 0);
    }

    #[test]
    fn feed_view_push() {
        let mut feed = FeedView::new();
        feed.push(FeedItem::system("Started"));
        assert_eq!(feed.len(), 1);
        assert!(!feed.is_empty());
    }

    #[test]
    fn feed_view_scroll_to_bottom() {
        let mut feed = FeedView::new();
        for i in 0..20 {
            feed.push(FeedItem::system(format!("Item {}", i)));
        }

        // Visible height of 10, should scroll to offset 10
        feed.scroll_to_bottom(10);
        assert_eq!(feed.scroll_offset, 10);
    }

    #[test]
    fn feed_view_scroll_to_bottom_small_feed() {
        let mut feed = FeedView::new();
        for i in 0..5 {
            feed.push(FeedItem::system(format!("Item {}", i)));
        }

        // Visible height of 10, feed only has 5 items
        feed.scroll_to_bottom(10);
        assert_eq!(feed.scroll_offset, 0);
    }

    #[test]
    fn feed_view_scroll_up() {
        let mut feed = FeedView::new();
        feed.scroll_offset = 10;
        feed.scroll_up(3);
        assert_eq!(feed.scroll_offset, 7);
    }

    #[test]
    fn feed_view_scroll_up_floor() {
        let mut feed = FeedView::new();
        feed.scroll_offset = 2;
        feed.scroll_up(5);
        assert_eq!(feed.scroll_offset, 0);
    }

    #[test]
    fn feed_view_scroll_down() {
        let mut feed = FeedView::new();
        for i in 0..20 {
            feed.push(FeedItem::system(format!("Item {}", i)));
        }
        feed.scroll_down(5, 10);
        assert_eq!(feed.scroll_offset, 5);
    }

    #[test]
    fn feed_view_scroll_down_capped() {
        let mut feed = FeedView::new();
        for i in 0..20 {
            feed.push(FeedItem::system(format!("Item {}", i)));
        }
        feed.scroll_down(100, 10);
        // Max offset is 20 - 10 = 10
        assert_eq!(feed.scroll_offset, 10);
    }

    #[test]
    fn feed_item_type_icons() {
        assert_eq!(FeedItemType::AgentReport.icon_and_style().0, "●");
        assert_eq!(FeedItemType::TaskStarted.icon_and_style().0, "▶");
        assert_eq!(FeedItemType::TaskCompleted.icon_and_style().0, "✓");
        assert_eq!(FeedItemType::Milestone.icon_and_style().0, "★");
        assert_eq!(FeedItemType::Error.icon_and_style().0, "✗");
        assert_eq!(FeedItemType::SystemNotice.icon_and_style().0, "ℹ");
    }
}
