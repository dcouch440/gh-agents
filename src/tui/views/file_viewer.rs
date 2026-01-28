//! File viewer widget with syntax highlighting and search.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};
use std::path::PathBuf;

/// Read-only file viewer with scrolling, line numbers, and search.
#[derive(Debug, Clone)]
pub struct FileViewer {
    /// File path being viewed.
    path: PathBuf,
    /// File content split into lines.
    lines: Vec<String>,
    /// Current scroll position (first visible line, 0-indexed).
    scroll_offset: usize,
    /// Cursor line position (for highlighting current line).
    cursor_line: usize,
    /// Search query (if any).
    search_query: Option<String>,
    /// Search match positions (line, column).
    search_matches: Vec<(usize, usize)>,
    /// Whether search input is active.
    search_active: bool,
    /// Current search input.
    search_input: String,
    /// Current match index.
    current_match: usize,
    /// Viewport height (updated during render).
    viewport_height: usize,
}

impl FileViewer {
    /// Create a new file viewer from path and content.
    pub fn new(path: PathBuf, content: String) -> Self {
        let lines: Vec<String> = content.lines().map(String::from).collect();
        Self {
            path,
            lines,
            scroll_offset: 0,
            cursor_line: 0,
            search_query: None,
            search_matches: Vec::new(),
            search_active: false,
            search_input: String::new(),
            current_match: 0,
            viewport_height: 20,
        }
    }

    /// Get the file path.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get the number of lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Get the current cursor line (1-indexed for display).
    pub fn cursor_line_display(&self) -> usize {
        self.cursor_line + 1
    }

    /// Get current scroll offset.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Check if search is active.
    pub fn is_search_active(&self) -> bool {
        self.search_active
    }

    /// Get search input.
    pub fn search_input(&self) -> &str {
        &self.search_input
    }

    /// Get search matches count.
    pub fn search_match_count(&self) -> usize {
        self.search_matches.len()
    }

    /// Get current match index (1-indexed for display).
    pub fn current_match_display(&self) -> usize {
        if self.search_matches.is_empty() {
            0
        } else {
            self.current_match + 1
        }
    }

    /// Scroll up by the given amount.
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        self.cursor_line = self.cursor_line.saturating_sub(amount);
    }

    /// Scroll down by the given amount.
    pub fn scroll_down(&mut self, amount: usize) {
        let max_scroll = self.lines.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max_scroll);
        self.cursor_line = (self.cursor_line + amount).min(max_scroll);
    }

    /// Move cursor up one line.
    pub fn cursor_up(&mut self) {
        self.cursor_line = self.cursor_line.saturating_sub(1);
        // Adjust scroll if cursor goes above viewport
        if self.cursor_line < self.scroll_offset {
            self.scroll_offset = self.cursor_line;
        }
    }

    /// Move cursor down one line.
    pub fn cursor_down(&mut self) {
        let max_line = self.lines.len().saturating_sub(1);
        self.cursor_line = (self.cursor_line + 1).min(max_line);
        // Adjust scroll if cursor goes below viewport
        if self.cursor_line >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = self.cursor_line.saturating_sub(self.viewport_height - 1);
        }
    }

    /// Page up.
    pub fn page_up(&mut self) {
        let amount = self.viewport_height.saturating_sub(2);
        self.scroll_up(amount);
    }

    /// Page down.
    pub fn page_down(&mut self) {
        let amount = self.viewport_height.saturating_sub(2);
        self.scroll_down(amount);
    }

    /// Jump to top of file.
    pub fn go_to_top(&mut self) {
        self.scroll_offset = 0;
        self.cursor_line = 0;
    }

    /// Jump to bottom of file.
    pub fn go_to_bottom(&mut self) {
        let max = self.lines.len().saturating_sub(1);
        self.cursor_line = max;
        self.scroll_offset = max.saturating_sub(self.viewport_height.saturating_sub(1));
    }

    /// Calculate width needed for line numbers.
    fn line_number_width(&self) -> usize {
        if self.lines.is_empty() {
            return 4;
        }
        let digits = ((self.lines.len() as f64).log10().floor() as usize) + 1;
        digits.max(3) + 1 // +1 for padding
    }

    /// Render the file viewer.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // Reserve space for status bar (1 line) and search bar if active (1 line)
        let search_height = if self.search_active { 1 } else { 0 };
        let content_height = area.height.saturating_sub(1 + search_height);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(content_height),
                Constraint::Length(search_height),
                Constraint::Length(1), // status bar
            ])
            .split(area);

        self.render_content_area(chunks[0], buf);

        if self.search_active {
            self.render_search_bar(chunks[1], buf);
        }

        self.render_status_bar(chunks[2], buf);
    }

    fn render_content_area(&self, area: Rect, buf: &mut Buffer) {
        let line_num_width = self.line_number_width() as u16;

        // Split into line numbers and content
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(line_num_width),
                Constraint::Min(1),
                Constraint::Length(1), // scrollbar
            ])
            .split(area);

        self.render_line_numbers(chunks[0], buf);
        self.render_content(chunks[1], buf);
        self.render_scrollbar(chunks[2], buf);
    }

    fn render_line_numbers(&self, area: Rect, buf: &mut Buffer) {
        let visible_lines = area.height as usize;
        let line_num_width = self.line_number_width();

        for (i, row) in (0..visible_lines).enumerate() {
            let line_idx = self.scroll_offset + i;
            if line_idx >= self.lines.len() {
                break;
            }

            let num = line_idx + 1; // 1-indexed display
            let style = if line_idx == self.cursor_line {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let num_str = format!("{:>width$}", num, width = line_num_width - 1);
            buf.set_string(area.x, area.y + row as u16, &num_str, style);
        }
    }

    fn render_content(&self, area: Rect, buf: &mut Buffer) {
        let visible_lines = area.height as usize;

        for (i, row) in (0..visible_lines).enumerate() {
            let line_idx = self.scroll_offset + i;
            if line_idx >= self.lines.len() {
                break;
            }

            let line = &self.lines[line_idx];
            let is_cursor_line = line_idx == self.cursor_line;

            // Build styled line with search highlights
            let styled_line = self.style_line(line, line_idx, is_cursor_line);

            // Render background for cursor line
            if is_cursor_line {
                for x in 0..area.width {
                    buf[(area.x + x, area.y + row as u16)].set_bg(Color::Rgb(40, 40, 50));
                }
            }

            // Render the line content
            let y = area.y + row as u16;
            let mut x = area.x;
            for span in styled_line.spans {
                let content = span.content.to_string();
                for ch in content.chars() {
                    if x >= area.right() {
                        break;
                    }
                    buf[(x, y)].set_char(ch).set_style(span.style);
                    x += 1;
                }
            }
        }
    }

    fn style_line(&self, line: &str, line_idx: usize, is_cursor_line: bool) -> Line<'static> {
        let base_style = if is_cursor_line {
            Style::default().bg(Color::Rgb(40, 40, 50))
        } else {
            Style::default()
        };

        // If no search, return plain line
        if self.search_query.is_none() || self.search_input.is_empty() {
            return Line::from(Span::styled(line.to_string(), base_style));
        }

        // Find matches on this line and highlight them
        let query = self.search_input.to_lowercase();
        let line_lower = line.to_lowercase();
        let mut spans = Vec::new();
        let mut last_end = 0;

        let mut col = 0;
        while let Some(pos) = line_lower[col..].find(&query) {
            let start = col + pos;
            let end = start + self.search_input.len();

            // Add text before match
            if start > last_end {
                spans.push(Span::styled(line[last_end..start].to_string(), base_style));
            }

            // Check if this is the current match
            let is_current =
                self.search_matches.get(self.current_match) == Some(&(line_idx, start));

            let match_style = if is_current {
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .bg(Color::Rgb(100, 100, 0))
                    .fg(Color::White)
            };

            spans.push(Span::styled(line[start..end].to_string(), match_style));
            last_end = end;
            col = end;
        }

        // Add remaining text
        if last_end < line.len() {
            spans.push(Span::styled(line[last_end..].to_string(), base_style));
        }

        if spans.is_empty() {
            Line::from(Span::styled(line.to_string(), base_style))
        } else {
            Line::from(spans)
        }
    }

    fn render_scrollbar(&self, area: Rect, buf: &mut Buffer) {
        if self.lines.is_empty() {
            return;
        }

        let mut state = ScrollbarState::new(self.lines.len()).position(self.scroll_offset);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        scrollbar.render(area, buf, &mut state);
    }

    fn render_status_bar(&self, area: Rect, buf: &mut Buffer) {
        let status = format!(
            " {} | Ln {}, Col 1 | {} lines | [Read Only]",
            self.path.display(),
            self.cursor_line + 1,
            self.lines.len()
        );

        // Clear the status bar area
        for x in 0..area.width {
            buf[(area.x + x, area.y)]
                .set_char(' ')
                .set_bg(Color::DarkGray);
        }

        let style = Style::default().bg(Color::DarkGray).fg(Color::White);
        buf.set_string(area.x, area.y, &status, style);
    }

    fn render_search_bar(&self, area: Rect, buf: &mut Buffer) {
        let match_info = if self.search_matches.is_empty() {
            if self.search_input.is_empty() {
                String::new()
            } else {
                "No matches".to_string()
            }
        } else {
            format!("{}/{}", self.current_match + 1, self.search_matches.len())
        };

        let search_text = if match_info.is_empty() {
            format!("Search: {}", self.search_input)
        } else {
            format!("Search: {} | {}", self.search_input, match_info)
        };

        // Clear the search bar area
        for x in 0..area.width {
            buf[(area.x + x, area.y)].set_char(' ').set_bg(Color::Blue);
        }

        let style = Style::default().bg(Color::Blue).fg(Color::White);
        buf.set_string(area.x, area.y, &search_text, style);
    }

    // Search functionality

    /// Toggle search mode.
    pub fn toggle_search(&mut self) {
        self.search_active = !self.search_active;
        if !self.search_active {
            self.search_query = None;
            self.search_matches.clear();
            self.search_input.clear();
        }
    }

    /// Add a character to search input.
    pub fn search_input_char(&mut self, c: char) {
        self.search_input.push(c);
        self.update_search();
    }

    /// Remove last character from search input.
    pub fn search_input_backspace(&mut self) {
        self.search_input.pop();
        self.update_search();
    }

    /// Update search results based on current input.
    fn update_search(&mut self) {
        self.search_matches.clear();

        if self.search_input.is_empty() {
            self.search_query = None;
            return;
        }

        self.search_query = Some(self.search_input.clone());
        let query_lower = self.search_input.to_lowercase();

        for (line_idx, line) in self.lines.iter().enumerate() {
            let line_lower = line.to_lowercase();
            let mut col = 0;
            while let Some(pos) = line_lower[col..].find(&query_lower) {
                self.search_matches.push((line_idx, col + pos));
                col += pos + self.search_input.len();
            }
        }

        if !self.search_matches.is_empty() {
            self.current_match = 0;
            self.jump_to_match(0);
        }
    }

    /// Jump to next search match.
    pub fn next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.current_match = (self.current_match + 1) % self.search_matches.len();
        self.jump_to_match(self.current_match);
    }

    /// Jump to previous search match.
    pub fn prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.current_match = if self.current_match == 0 {
            self.search_matches.len() - 1
        } else {
            self.current_match - 1
        };
        self.jump_to_match(self.current_match);
    }

    fn jump_to_match(&mut self, idx: usize) {
        if let Some(&(line, _col)) = self.search_matches.get(idx) {
            self.cursor_line = line;
            // Ensure match is visible
            if line < self.scroll_offset {
                self.scroll_offset = line;
            } else if line >= self.scroll_offset + self.viewport_height {
                self.scroll_offset = line.saturating_sub(self.viewport_height / 2);
            }
        }
    }

    /// Update viewport height (call this with actual render area height).
    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
    }
}

impl Default for FileViewer {
    fn default() -> Self {
        Self::new(PathBuf::from("untitled"), String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_viewer() -> FileViewer {
        let content = (1..=100)
            .map(|i| format!("Line {}: Some content here", i))
            .collect::<Vec<_>>()
            .join("\n");
        FileViewer::new(PathBuf::from("test.rs"), content)
    }

    #[test]
    fn new_viewer_starts_at_top() {
        let viewer = sample_viewer();
        assert_eq!(viewer.scroll_offset(), 0);
        assert_eq!(viewer.cursor_line_display(), 1);
    }

    #[test]
    fn line_count_is_correct() {
        let viewer = sample_viewer();
        assert_eq!(viewer.line_count(), 100);
    }

    #[test]
    fn scroll_down_increases_offset() {
        let mut viewer = sample_viewer();
        viewer.scroll_down(5);
        assert_eq!(viewer.scroll_offset(), 5);
    }

    #[test]
    fn scroll_up_decreases_offset() {
        let mut viewer = sample_viewer();
        viewer.scroll_down(10);
        viewer.scroll_up(3);
        assert_eq!(viewer.scroll_offset(), 7);
    }

    #[test]
    fn scroll_up_floors_at_zero() {
        let mut viewer = sample_viewer();
        viewer.scroll_down(5);
        viewer.scroll_up(10);
        assert_eq!(viewer.scroll_offset(), 0);
    }

    #[test]
    fn scroll_down_caps_at_max() {
        let mut viewer = sample_viewer();
        viewer.scroll_down(200);
        assert_eq!(viewer.scroll_offset(), 99); // 100 lines - 1
    }

    #[test]
    fn cursor_down_moves_cursor() {
        let mut viewer = sample_viewer();
        viewer.cursor_down();
        assert_eq!(viewer.cursor_line_display(), 2);
    }

    #[test]
    fn cursor_up_moves_cursor() {
        let mut viewer = sample_viewer();
        viewer.cursor_down();
        viewer.cursor_down();
        viewer.cursor_up();
        assert_eq!(viewer.cursor_line_display(), 2);
    }

    #[test]
    fn cursor_up_floors_at_zero() {
        let mut viewer = sample_viewer();
        viewer.cursor_up();
        assert_eq!(viewer.cursor_line_display(), 1);
    }

    #[test]
    fn go_to_top_resets_position() {
        let mut viewer = sample_viewer();
        viewer.scroll_down(50);
        viewer.go_to_top();
        assert_eq!(viewer.scroll_offset(), 0);
        assert_eq!(viewer.cursor_line_display(), 1);
    }

    #[test]
    fn go_to_bottom_goes_to_last_line() {
        let mut viewer = sample_viewer();
        viewer.set_viewport_height(20);
        viewer.go_to_bottom();
        assert_eq!(viewer.cursor_line_display(), 100);
    }

    #[test]
    fn line_number_width_adjusts_for_line_count() {
        let content = "line1\nline2\nline3";
        let viewer = FileViewer::new(PathBuf::from("test.txt"), content.to_string());
        assert_eq!(viewer.line_number_width(), 4); // min 3 + 1 padding

        let content = (1..=1000)
            .map(|i| format!("Line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let viewer = FileViewer::new(PathBuf::from("test.txt"), content);
        assert_eq!(viewer.line_number_width(), 5); // 4 digits + 1 padding
    }

    #[test]
    fn search_toggle_activates_deactivates() {
        let mut viewer = sample_viewer();
        assert!(!viewer.is_search_active());
        viewer.toggle_search();
        assert!(viewer.is_search_active());
        viewer.toggle_search();
        assert!(!viewer.is_search_active());
    }

    #[test]
    fn search_input_updates_query() {
        let mut viewer = sample_viewer();
        viewer.toggle_search();
        viewer.search_input_char('L');
        viewer.search_input_char('i');
        viewer.search_input_char('n');
        viewer.search_input_char('e');
        assert_eq!(viewer.search_input(), "Line");
    }

    #[test]
    fn search_finds_matches() {
        let mut viewer = sample_viewer();
        viewer.toggle_search();
        viewer.search_input_char('L');
        viewer.search_input_char('i');
        viewer.search_input_char('n');
        viewer.search_input_char('e');
        // Each line contains "Line", so 100 matches
        assert_eq!(viewer.search_match_count(), 100);
    }

    #[test]
    fn search_next_match_cycles() {
        let mut viewer = sample_viewer();
        viewer.toggle_search();
        viewer.search_input_char('L');
        viewer.search_input_char('i');
        viewer.search_input_char('n');
        viewer.search_input_char('e');

        assert_eq!(viewer.current_match_display(), 1);
        viewer.next_match();
        assert_eq!(viewer.current_match_display(), 2);
    }

    #[test]
    fn search_prev_match_cycles_backwards() {
        let mut viewer = sample_viewer();
        viewer.toggle_search();
        viewer.search_input_char('L');
        viewer.search_input_char('i');
        viewer.search_input_char('n');
        viewer.search_input_char('e');

        viewer.prev_match();
        assert_eq!(viewer.current_match_display(), 100); // wraps to last
    }

    #[test]
    fn search_backspace_removes_char() {
        let mut viewer = sample_viewer();
        viewer.toggle_search();
        viewer.search_input_char('a');
        viewer.search_input_char('b');
        viewer.search_input_backspace();
        assert_eq!(viewer.search_input(), "a");
    }

    #[test]
    fn search_deactivate_clears_state() {
        let mut viewer = sample_viewer();
        viewer.toggle_search();
        viewer.search_input_char('L');
        viewer.search_input_char('i');
        viewer.toggle_search(); // deactivate
        assert!(!viewer.is_search_active());
        assert!(viewer.search_input().is_empty());
        assert_eq!(viewer.search_match_count(), 0);
    }

    #[test]
    fn empty_file_viewer() {
        let viewer = FileViewer::new(PathBuf::from("empty.txt"), String::new());
        assert_eq!(viewer.line_count(), 0);
        assert_eq!(viewer.scroll_offset(), 0);
    }

    #[test]
    fn default_viewer() {
        let viewer = FileViewer::default();
        assert_eq!(viewer.path().to_str().unwrap(), "untitled");
        assert_eq!(viewer.line_count(), 0);
    }
}
