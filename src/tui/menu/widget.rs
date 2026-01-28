//! Menu widget for ratatui rendering

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::types::{Menu, MenuItemType, MenuState, MenuStatus};

/// Widget for rendering a menu popup
pub struct MenuWidget<'a> {
    menu: &'a Menu,
    state: &'a MenuState,
    status: &'a MenuStatus,
}

impl<'a> MenuWidget<'a> {
    /// Create a new menu widget
    pub fn new(menu: &'a Menu, state: &'a MenuState, status: &'a MenuStatus) -> Self {
        Self {
            menu,
            state,
            status,
        }
    }
}

impl Widget for MenuWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Calculate menu size and center it
        let (width, height) = menu_size(self.menu, self.status);
        let menu_area = centered_rect(width, height, area);

        // Clear the area behind the menu
        for y in menu_area.y..menu_area.bottom() {
            for x in menu_area.x..menu_area.right() {
                buf[(x, y)].reset();
            }
        }

        // Draw outer border with title
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", self.menu.title))
            .style(Style::default().fg(Color::White));
        let inner = block.inner(menu_area);
        block.render(menu_area, buf);

        // Build content lines
        let mut lines: Vec<Line> = Vec::new();

        // Status header lines
        if !self.status.production_state.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("Production: {}", self.status.production_state),
                Style::default().fg(Color::Cyan),
            )));
        }
        if !self.status.current_milestone.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("Milestone: {}", self.status.current_milestone),
                Style::default().fg(Color::Cyan),
            )));
        }
        if self.status.pending_changes > 0 {
            lines.push(Line::from(Span::styled(
                format!("Pending: {} changes", self.status.pending_changes),
                Style::default().fg(Color::Yellow),
            )));
        }

        // Add separator if we have status lines
        if !lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "─".repeat(inner.width.saturating_sub(2) as usize),
                Style::default().fg(Color::DarkGray),
            )));
        }

        // Track which selectable item index we're on
        let mut selectable_index = 0;

        // Menu items
        for item in &self.menu.items {
            match &item.item_type {
                MenuItemType::Separator => {
                    lines.push(Line::from(Span::styled(
                        "─".repeat(inner.width.saturating_sub(2) as usize),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                _ => {
                    let is_selected =
                        item.is_selectable() && selectable_index == self.state.selected_index;

                    // Build the line
                    let mut spans = Vec::new();

                    // Selection indicator
                    if is_selected {
                        spans.push(Span::styled(
                            "> ",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        spans.push(Span::raw("  "));
                    }

                    // Label style based on enabled state
                    let label_style = if !item.enabled {
                        Style::default().fg(Color::DarkGray)
                    } else if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    // Calculate available width for label
                    let prefix_width = 2; // "> " or "  "
                    let suffix_width = 4; // " → " or " X " (shortcut) or "   "
                    let label_max_width =
                        inner.width.saturating_sub(prefix_width + suffix_width) as usize;

                    // Truncate label if needed
                    let label = if item.label.len() > label_max_width {
                        format!("{}...", &item.label[..label_max_width.saturating_sub(3)])
                    } else {
                        format!("{:width$}", item.label, width = label_max_width)
                    };

                    spans.push(Span::styled(label, label_style));

                    // Right side indicator
                    match &item.item_type {
                        MenuItemType::Submenu(_) => {
                            spans.push(Span::styled(" →", Style::default().fg(Color::DarkGray)));
                        }
                        MenuItemType::Back => {
                            spans.push(Span::styled(" ←", Style::default().fg(Color::DarkGray)));
                        }
                        MenuItemType::Action(_) => {
                            if let Some(shortcut) = item.shortcut {
                                spans.push(Span::styled(
                                    format!(" {}", shortcut),
                                    Style::default().fg(Color::DarkGray),
                                ));
                            } else {
                                spans.push(Span::raw("  "));
                            }
                        }
                        MenuItemType::Separator => {}
                    }

                    lines.push(Line::from(spans));

                    // Increment selectable index if this item is selectable
                    if item.is_selectable() {
                        selectable_index += 1;
                    }
                }
            }
        }

        // Render as a paragraph
        let paragraph = Paragraph::new(lines);
        paragraph.render(inner, buf);
    }
}

/// Calculate required widget size based on menu content
pub fn menu_size(menu: &Menu, status: &MenuStatus) -> (u16, u16) {
    // Calculate width based on longest item + prefix + suffix + border
    let mut max_width: u16 = menu.title.len() as u16 + 4; // title + " Menu " padding

    // Check status lines
    if !status.production_state.is_empty() {
        let len = format!("Production: {}", status.production_state).len() as u16;
        max_width = max_width.max(len);
    }
    if !status.current_milestone.is_empty() {
        let len = format!("Milestone: {}", status.current_milestone).len() as u16;
        max_width = max_width.max(len);
    }
    if status.pending_changes > 0 {
        let len = format!("Pending: {} changes", status.pending_changes).len() as u16;
        max_width = max_width.max(len);
    }

    // Check menu items
    for item in &menu.items {
        if !matches!(item.item_type, MenuItemType::Separator) {
            // "> label →" or "> label X"
            let item_width = 2 + item.label.len() as u16 + 3;
            max_width = max_width.max(item_width);
        }
    }

    // Add border padding
    let width = max_width + 4;

    // Calculate height
    let mut height: u16 = 2; // borders

    // Status lines
    let status_lines = [
        !status.production_state.is_empty(),
        !status.current_milestone.is_empty(),
        status.pending_changes > 0,
    ]
    .iter()
    .filter(|&&b| b)
    .count() as u16;

    if status_lines > 0 {
        height += status_lines + 1; // status lines + separator
    }

    // Menu items (each item is 1 line)
    height += menu.items.len() as u16;

    (width, height)
}

/// Center a rect within another rect
pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::menu::{Menu, MenuAction, MenuItem};

    fn test_menu() -> Menu {
        Menu::new("test", "Test Menu")
            .add_item(MenuItem::action("item1", "First Item", MenuAction::Quit))
            .add_item(MenuItem::separator())
            .add_item(MenuItem::submenu("sub", "Submenu", "other"))
    }

    #[test]
    fn menu_size_includes_items() {
        let menu = test_menu();
        let status = MenuStatus::default();

        let (width, height) = menu_size(&menu, &status);

        // Should accommodate title and items
        assert!(width >= 15);
        // Header (2) + 3 items
        assert_eq!(height, 5);
    }

    #[test]
    fn menu_size_includes_status() {
        let menu = test_menu();
        let status = MenuStatus::new()
            .with_production_state("Running")
            .with_milestone("M3");

        let (width, height) = menu_size(&menu, &status);

        // Should be taller to include status lines
        // Header (2) + 2 status lines + separator + 3 items = 8
        assert_eq!(height, 8);
        // Width should accommodate status text
        assert!(width >= "Production: Running".len() as u16 + 4);
    }

    #[test]
    fn centered_rect_centers_in_area() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(20, 10, area);

        assert_eq!(centered.x, 40);
        assert_eq!(centered.y, 20);
        assert_eq!(centered.width, 20);
        assert_eq!(centered.height, 10);
    }

    #[test]
    fn centered_rect_clamps_to_area() {
        let area = Rect::new(0, 0, 20, 10);
        let centered = centered_rect(100, 50, area);

        assert_eq!(centered.width, 20);
        assert_eq!(centered.height, 10);
    }

    #[test]
    fn widget_can_be_created() {
        let menu = test_menu();
        let state = MenuState::default();
        let status = MenuStatus::default();

        let _widget = MenuWidget::new(&menu, &state, &status);
    }

    #[test]
    fn widget_renders_without_panic() {
        let menu = test_menu();
        let mut state = MenuState::default();
        state.open();
        let status = MenuStatus::new().with_production_state("Idle");

        let widget = MenuWidget::new(&menu, &state, &status);

        // Create a test buffer
        let area = Rect::new(0, 0, 40, 20);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf);

        // Check that something was rendered - collect all content
        let mut content = String::new();
        for y in 0..area.height {
            for x in 0..area.width {
                content.push_str(buf[(x, y)].symbol());
            }
        }

        // The menu should contain some recognizable content
        assert!(
            content.contains("Test") || content.contains("First") || content.contains("─"),
            "Buffer should contain menu content, got: {}",
            content.chars().take(200).collect::<String>()
        );
    }
}
