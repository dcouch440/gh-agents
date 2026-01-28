//! Tasks view showing the task queue

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, Widget},
};

use crate::types::{Priority, Task, TaskStatus};

/// View displaying the task queue
pub struct TasksView {
    pub tasks: Vec<Task>,
    pub selected: Option<usize>,
    pub scroll_offset: usize,
}

impl Default for TasksView {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            selected: None,
            scroll_offset: 0,
        }
    }
}

impl TasksView {
    pub fn new(tasks: Vec<Task>) -> Self {
        Self {
            tasks,
            selected: None,
            scroll_offset: 0,
        }
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self, visible_height: usize) {
        let max_offset = self.tasks.len().saturating_sub(visible_height);
        if self.scroll_offset < max_offset {
            self.scroll_offset += 1;
        }
    }

    fn status_style(status: &TaskStatus) -> (&'static str, Style) {
        match status {
            TaskStatus::Pending => ("Pend", Style::default().fg(Color::Gray)),
            TaskStatus::InProgress => ("Work", Style::default().fg(Color::Blue)),
            TaskStatus::Review => ("Review", Style::default().fg(Color::Yellow)),
            TaskStatus::Completed => ("Done", Style::default().fg(Color::Green)),
            TaskStatus::Failed => ("Fail", Style::default().fg(Color::Red)),
        }
    }

    fn priority_style(priority: &Priority) -> Style {
        match priority {
            Priority::Low => Style::default().fg(Color::DarkGray),
            Priority::Normal => Style::default(),
            Priority::High => Style::default().fg(Color::Yellow),
            Priority::Urgent => Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        }
    }
}

impl Widget for TasksView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.tasks.is_empty() {
            let msg = "No tasks in queue. Start by chatting with the orchestrator.";
            buf.set_string(
                area.x + 2,
                area.y + 2,
                msg,
                Style::default().fg(Color::DarkGray),
            );
            return;
        }

        let header = Row::new(vec!["ID", "Title", "Status", "Priority", "Agent"])
            .style(Style::default().add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let visible_height = area.height.saturating_sub(4) as usize;

        let rows: Vec<Row> = self
            .tasks
            .iter()
            .skip(self.scroll_offset)
            .take(visible_height)
            .enumerate()
            .map(|(idx, task)| {
                let (status_text, status_style) = Self::status_style(&task.status);
                let priority_style = Self::priority_style(&task.priority);

                let id = task.id.0.to_string()[..8].to_string();
                let title = if task.title.len() > 35 {
                    format!("{}...", &task.title[..32])
                } else {
                    task.title.clone()
                };
                let agent = task
                    .assigned_agent
                    .as_ref()
                    .map(|a| a.0.to_string()[..6].to_string())
                    .unwrap_or_else(|| "-".to_string());

                let row_style = if self.selected == Some(idx + self.scroll_offset) {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                Row::new(vec![
                    Cell::from(id),
                    Cell::from(title),
                    Cell::from(status_text).style(status_style),
                    Cell::from(format!("{:?}", task.priority)).style(priority_style),
                    Cell::from(agent),
                ])
                .style(row_style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(10),  // ID
                Constraint::Min(20),     // Title
                Constraint::Length(8),   // Status
                Constraint::Length(10),  // Priority
                Constraint::Length(10),  // Agent
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Tasks ({}) ", self.tasks.len())),
        );

        Widget::render(table, area, buf);

        // Show scroll indicator if needed
        if self.tasks.len() > visible_height {
            let scroll_info = format!(
                " {}-{} of {} ",
                self.scroll_offset + 1,
                (self.scroll_offset + visible_height).min(self.tasks.len()),
                self.tasks.len()
            );
            buf.set_string(
                area.right().saturating_sub(scroll_info.len() as u16 + 2),
                area.bottom().saturating_sub(1),
                &scroll_info,
                Style::default().fg(Color::DarkGray),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentTier;

    fn mock_task(title: &str, status: TaskStatus, priority: Priority) -> Task {
        let mut task = Task::new(title, AgentTier::Worker);
        task.status = status;
        task.priority = priority;
        task
    }

    #[test]
    fn tasks_view_default_empty() {
        let view = TasksView::default();
        assert!(view.tasks.is_empty());
        assert_eq!(view.scroll_offset, 0);
        assert!(view.selected.is_none());
    }

    #[test]
    fn tasks_view_with_tasks() {
        let tasks = vec![
            mock_task("Task 1", TaskStatus::Pending, Priority::Normal),
            mock_task("Task 2", TaskStatus::InProgress, Priority::High),
        ];
        let view = TasksView::new(tasks);
        assert_eq!(view.tasks.len(), 2);
    }

    #[test]
    fn scroll_respects_bounds() {
        let tasks = vec![
            mock_task("Task 1", TaskStatus::Pending, Priority::Normal),
            mock_task("Task 2", TaskStatus::Pending, Priority::Normal),
        ];
        let mut view = TasksView::new(tasks);

        // Can't scroll up from 0
        view.scroll_up();
        assert_eq!(view.scroll_offset, 0);

        // Scroll down respects max
        view.scroll_down(10); // visible height larger than tasks
        assert_eq!(view.scroll_offset, 0);
    }

    #[test]
    fn status_styles_are_distinct() {
        let (_, pending_style) = TasksView::status_style(&TaskStatus::Pending);
        let (_, progress_style) = TasksView::status_style(&TaskStatus::InProgress);
        let (_, done_style) = TasksView::status_style(&TaskStatus::Completed);
        let (_, failed_style) = TasksView::status_style(&TaskStatus::Failed);

        // Each status should have a different foreground color
        assert_ne!(pending_style, progress_style);
        assert_ne!(done_style, failed_style);
    }

    #[test]
    fn priority_styles_are_distinct() {
        let low = TasksView::priority_style(&Priority::Low);
        let normal = TasksView::priority_style(&Priority::Normal);
        let high = TasksView::priority_style(&Priority::High);
        let urgent = TasksView::priority_style(&Priority::Urgent);

        assert_ne!(low, normal);
        assert_ne!(high, urgent);
    }
}
