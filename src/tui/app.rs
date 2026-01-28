//! Main TUI application state and logic.
//!
//! The App struct manages:
//! - Current mode (Normal, Refactor)
//! - Current view (Feed, Main, Logs, etc.)
//! - User input processing
//! - Command routing

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::tui::input::InputBar;
use crate::tui::layout::{AppLayout, HeaderBar};
use crate::tui::views::{
    AgentsView, ChatMessage, ChatView, CostsView, FeedItem, FeedView, HomeView, LogEntry, LogsView,
    TasksView,
};
use std::io::{self, Stdout};
use std::panic;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::db::{get_milestone_limit, set_milestone_limit};
use crate::orchestration::Scheduler;
use crate::refactor::RefactorAgent;
use crate::tui::commands::{generate_help_text, Command, CommandResult};
use crate::tui::menu::{build_menu_tree, centered_rect, menu_size, MenuAction, MenuController, MenuStatus, MenuWidget};
use crate::tui::mode::{AppMode, RefactorModeState};

/// Terminal type alias for convenience
pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal for TUI rendering.
///
/// Enables raw mode and enters alternate screen so the TUI doesn't
/// overwrite terminal history.
pub fn init_terminal() -> io::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Restore the terminal to its original state.
///
/// Disables raw mode, leaves alternate screen, and shows cursor.
pub fn restore_terminal(terminal: &mut Tui) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Install a panic hook that restores the terminal before printing panic info.
///
/// Without this, a panic would leave the terminal in raw mode with the
/// alternate screen still active, making it unusable.
pub fn install_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Best effort to restore terminal
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}

/// Current view being displayed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Home,
    Feed,
    Main,
    Logs,
    Tasks,
    Agents,
    Costs,
}

impl View {
    pub fn name(&self) -> &'static str {
        match self {
            View::Home => "Home",
            View::Feed => "Feed",
            View::Main => "Main",
            View::Logs => "Logs",
            View::Tasks => "Tasks",
            View::Agents => "Agents",
            View::Costs => "Costs",
        }
    }
}

/// Main TUI application
pub struct App {
    /// Current operating mode
    mode: AppMode,
    /// Current view
    view: View,
    /// Scheduler for controlling production
    scheduler: Arc<RwLock<Scheduler>>,
    /// Database pool
    pool: sqlx::SqlitePool,
    /// Refactor agent (created when entering refactor mode)
    refactor_agent: Option<RefactorAgent>,
    /// Project root path
    project_root: PathBuf,
    /// Whether the app should quit
    should_quit: bool,
    /// Message buffer for displaying messages to the user
    message: Option<String>,
    /// Activity feed
    feed: FeedView,
    /// Chat conversation with orchestrator
    chat: ChatView,
    /// Pending streaming response from orchestrator
    pending_response: Option<ChatMessage>,
    /// Technical logs
    logs: LogsView,
    /// Cached milestone limit for quick access
    cached_milestone_limit: Option<u8>,
    /// Whether the menu is currently open
    menu_open: bool,
}

impl App {
    /// Create a new application
    pub fn new(
        scheduler: Arc<RwLock<Scheduler>>,
        pool: sqlx::SqlitePool,
        project_root: PathBuf,
    ) -> Self {
        // Initialize feed with startup message
        let mut feed = FeedView::new();
        feed.push(FeedItem::system("nexor started"));

        // Initialize logs with startup messages
        let mut logs = LogsView::new();
        logs.push(LogEntry::info("nexor::tui", "Terminal initialized"));
        logs.push(LogEntry::info("nexor::db", "Database connected"));
        logs.push(LogEntry::info("nexor::app", "Application started"));

        Self {
            mode: AppMode::Normal,
            view: View::Home,
            scheduler,
            pool,
            refactor_agent: None,
            project_root,
            should_quit: false,
            message: None,
            feed,
            chat: ChatView::new(),
            pending_response: None,
            logs,
            cached_milestone_limit: None,
            menu_open: false,
        }
    }

    /// Get the current mode
    pub fn mode(&self) -> &AppMode {
        &self.mode
    }

    /// Get the current view
    pub fn view(&self) -> View {
        self.view
    }

    /// Check if the app should quit
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Get the current message (if any)
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Clear the current message
    pub fn clear_message(&mut self) {
        self.message = None;
    }

    /// Set a message to display
    pub fn set_message(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
    }

    /// Add an item to the activity feed.
    pub fn push_feed_item(&mut self, item: FeedItem) {
        self.feed.push(item);
    }

    /// Get a reference to the feed.
    pub fn feed(&self) -> &FeedView {
        &self.feed
    }

    /// Get a reference to the chat.
    pub fn chat(&self) -> &ChatView {
        &self.chat
    }

    /// Get a reference to the logs.
    pub fn logs(&self) -> &LogsView {
        &self.logs
    }

    /// Add a log entry.
    pub fn push_log(&mut self, entry: LogEntry) {
        self.logs.push(entry);
    }

    /// Submit a chat message to the orchestrator.
    ///
    /// Adds the user message to chat history and generates a mock response
    /// (real orchestrator integration happens in M3).
    fn submit_chat_message(&mut self, content: String) {
        // Add user message to history
        self.chat.push(ChatMessage::user(&content));

        // Add to feed
        self.feed.push(FeedItem::new(
            "You",
            &content,
            crate::tui::views::FeedItemType::AgentReport,
        ));

        // Mock response - in real implementation, this would go through the orchestrator
        let response = format!(
            "I received your message: \"{}\". (Mock response - orchestrator integration pending)",
            content
        );
        self.chat.push(ChatMessage::orchestrator(&response));

        // Add orchestrator response to feed
        self.feed.push(FeedItem::new(
            "Orchestrator",
            "Responded to user message",
            crate::tui::views::FeedItemType::AgentReport,
        ));
    }

    /// Get the status bar text
    pub fn status_bar_text(&self) -> String {
        let mode_indicator = self.mode.status_indicator();
        let view_name = self.view.name();

        if mode_indicator.is_empty() {
            format!("{} | Type /help for commands", view_name)
        } else {
            let state_summary = match &self.mode {
                AppMode::Refactor(state) => format!(" | {}", state.summary()),
                _ => String::new(),
            };
            format!(
                "{} {} | {}{}",
                mode_indicator, view_name, "Type /exit to leave", state_summary
            )
        }
    }

    /// Process user input
    ///
    /// Returns true if the input was handled, false otherwise.
    pub async fn handle_input(&mut self, input: &str) -> Result<bool> {
        let input = input.trim();
        if input.is_empty() {
            return Ok(false);
        }

        // Check for slash commands
        if let Some(cmd) = Command::parse(input) {
            let result = self.execute_command(cmd).await?;
            self.handle_command_result(result);
            return Ok(true);
        }

        // In refactor mode, treat non-command input as conversation
        if let AppMode::Refactor(_) = &self.mode {
            self.handle_refactor_input(input).await?;
            return Ok(true);
        }

        // In normal mode, handle chat messages
        match self.view {
            View::Main => {
                // Submit to chat
                self.submit_chat_message(input.to_string());
            }
            View::Home => {
                // From home, transition to Main and submit
                self.view = View::Main;
                self.submit_chat_message(input.to_string());
            }
            _ => {
                // Other views: just show the input as a message
                self.set_message(format!("Input: {}", input));
            }
        }
        Ok(true)
    }

    /// Execute a slash command
    async fn execute_command(&mut self, cmd: Command) -> Result<CommandResult> {
        match cmd {
            Command::Refactor => self.enter_refactor_mode().await,
            Command::Exit => self.handle_exit().await,
            Command::Help => Ok(CommandResult::Help(generate_help_text())),
            Command::Quit => Ok(CommandResult::Quit),
            Command::Menu => {
                self.menu_open = true;
                Ok(CommandResult::Success("Opening menu...".to_string()))
            }
            Command::Home => {
                self.view = View::Home;
                Ok(CommandResult::ViewChanged)
            }
            Command::Feed => {
                self.view = View::Feed;
                Ok(CommandResult::ViewChanged)
            }
            Command::Main => {
                self.view = View::Main;
                Ok(CommandResult::ViewChanged)
            }
            Command::Logs => {
                self.view = View::Logs;
                Ok(CommandResult::ViewChanged)
            }
            Command::Tasks => {
                self.view = View::Tasks;
                Ok(CommandResult::ViewChanged)
            }
            Command::Agents => {
                self.view = View::Agents;
                Ok(CommandResult::ViewChanged)
            }
            Command::Costs => {
                self.view = View::Costs;
                Ok(CommandResult::ViewChanged)
            }
            Command::Unknown(name) => {
                Ok(CommandResult::Error(format!("Unknown command: /{}", name)))
            }
        }
    }

    /// Handle the result of a command execution
    fn handle_command_result(&mut self, result: CommandResult) {
        match result {
            CommandResult::Success(msg) => self.set_message(msg),
            CommandResult::Error(msg) => self.set_message(format!("Error: {}", msg)),
            CommandResult::ModeChanged => {
                // Mode change message is set by the mode change function
            }
            CommandResult::ViewChanged => {
                self.set_message(format!("Switched to {} view", self.view.name()));
            }
            CommandResult::Quit => {
                self.should_quit = true;
            }
            CommandResult::Help(text) => {
                self.set_message(text);
            }
        }
    }

    /// Enter refactor mode
    async fn enter_refactor_mode(&mut self) -> Result<CommandResult> {
        if self.mode.is_refactor() {
            return Ok(CommandResult::Error(
                "Already in refactor mode. Type /exit to leave.".to_string(),
            ));
        }

        // Create refactor agent and start session
        let mut agent = RefactorAgent::new(self.scheduler.clone(), self.pool.clone());
        let session = agent.start_session().await?;

        let state = RefactorModeState::from_session(session.clone());
        self.mode = AppMode::Refactor(state);
        self.refactor_agent = Some(agent);

        self.set_message(
            "Entered refactor mode. Describe the changes you want to make, or type /exit to leave.",
        );
        Ok(CommandResult::ModeChanged)
    }

    /// Handle /exit command based on current mode
    async fn handle_exit(&mut self) -> Result<CommandResult> {
        match &self.mode {
            AppMode::Normal => {
                // In normal mode, exit quits the app
                Ok(CommandResult::Quit)
            }
            AppMode::Refactor(_) => self.exit_refactor_mode().await,
        }
    }

    /// Exit refactor mode
    async fn exit_refactor_mode(&mut self) -> Result<CommandResult> {
        if let Some(agent) = &mut self.refactor_agent {
            agent.end_session().await?;
        }

        self.mode = AppMode::Normal;
        self.refactor_agent = None;
        self.set_message("Exited refactor mode. Production resumed.");
        Ok(CommandResult::ModeChanged)
    }

    /// Handle input while in refactor mode
    async fn handle_refactor_input(&mut self, input: &str) -> Result<()> {
        let agent = self
            .refactor_agent
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Refactor agent not initialized"))?;

        // Analyze intent
        let analysis = agent.analyze_intent_simple(input);

        // Update mode state if we have a session
        if let Some(session) = agent.session() {
            if let AppMode::Refactor(state) = &mut self.mode {
                state.update_from_session(session);
            }
        }

        // Handle based on intent
        match analysis.intent {
            crate::types::RefactorIntent::HaltNow => {
                agent.halt_production().await?;
                if let AppMode::Refactor(state) = &mut self.mode {
                    state.production_halted = true;
                }
                self.set_message(analysis.response);
            }
            crate::types::RefactorIntent::ExitRefactor => {
                self.exit_refactor_mode().await?;
            }
            _ => {
                // For other intents, show the response
                self.set_message(analysis.response);
            }
        }

        Ok(())
    }

    /// Get the refactor agent (if in refactor mode)
    pub fn refactor_agent(&self) -> Option<&RefactorAgent> {
        self.refactor_agent.as_ref()
    }

    /// Get mutable access to the refactor agent
    pub fn refactor_agent_mut(&mut self) -> Option<&mut RefactorAgent> {
        self.refactor_agent.as_mut()
    }

    // =====================
    // Menu-related methods
    // =====================

    /// Generate status info for menu header
    pub fn get_menu_status(&self) -> MenuStatus {
        MenuStatus::new()
            .with_production_state(self.production_state_string())
            .with_milestone(self.milestone_status_string())
            .with_pending_changes(self.pending_change_count())
    }

    /// Get production state as a string
    fn production_state_string(&self) -> String {
        match &self.mode {
            AppMode::Normal => {
                if self.is_production_paused() {
                    "Paused".to_string()
                } else {
                    "Running".to_string()
                }
            }
            AppMode::Refactor(_) => "Refactor Mode".to_string(),
        }
    }

    /// Get milestone status string for display
    fn milestone_status_string(&self) -> String {
        let current = self.current_milestone();
        match self.milestone_limit() {
            Some(limit) => format!("{} (limit: M{})", current, limit),
            None => current,
        }
    }

    /// Get count of pending changes
    fn pending_change_count(&self) -> usize {
        match &self.mode {
            AppMode::Refactor(state) => state.pending_changes,
            AppMode::Normal => 0,
        }
    }

    /// Check if production is paused
    fn is_production_paused(&self) -> bool {
        // Check scheduler state - will need to make this async or cache the state
        false // Default to not paused until scheduler integration
    }

    /// Get current milestone identifier
    fn current_milestone(&self) -> String {
        // This would come from the scheduler or current work
        "M9".to_string() // Placeholder
    }

    /// Get cached milestone limit
    fn milestone_limit(&self) -> Option<u8> {
        self.cached_milestone_limit
    }

    /// Load milestone limit from database
    pub async fn load_milestone_limit(&mut self) -> Result<()> {
        self.cached_milestone_limit = get_milestone_limit(&self.pool).await?;
        Ok(())
    }

    /// Execute a menu action
    async fn execute_menu_action(&mut self, action: MenuAction) -> Result<()> {
        match action {
            // Production control
            MenuAction::StartProduction => {
                let scheduler = self.scheduler.read().await;
                scheduler.resume().await?;
                drop(scheduler);
                self.set_message("Production resumed");
            }
            MenuAction::PauseProduction => {
                let scheduler = self.scheduler.read().await;
                scheduler.pause_for_refactor().await?;
                drop(scheduler);
                self.set_message("Production paused");
            }
            MenuAction::SetMilestoneLimit(milestone) => {
                set_milestone_limit(&self.pool, Some(milestone)).await?;
                self.cached_milestone_limit = Some(milestone);
                self.set_message(format!("Milestone limit set to M{}", milestone));
            }
            MenuAction::ClearMilestoneLimit => {
                set_milestone_limit(&self.pool, None).await?;
                self.cached_milestone_limit = None;
                self.set_message("Milestone limit cleared");
            }

            // Refactor mode
            MenuAction::ExitRefactorMode => {
                self.exit_refactor_mode().await?;
            }
            MenuAction::CancelRefactor => {
                // Cancel without applying - just exit
                if let Some(agent) = &mut self.refactor_agent {
                    agent.end_session().await?;
                }
                self.mode = AppMode::Normal;
                self.refactor_agent = None;
                self.set_message("Refactor cancelled");
            }
            MenuAction::ApplyChanges => {
                let applied = self.apply_refactor_changes().await?;
                self.set_message(format!("Applied {} changes", applied.len()));
            }
            MenuAction::DiscardChanges => {
                if let AppMode::Refactor(state) = &mut self.mode {
                    state.pending_changes = 0;
                }
                self.set_message("Pending changes discarded");
            }
            MenuAction::ReviewChanges => {
                // Switch to a view that shows pending changes
                self.set_message("Review changes (not yet implemented)");
            }

            // Navigation
            MenuAction::GoToView(view) => {
                self.view = view;
                self.set_message(format!("Switched to {} view", view.name()));
            }

            // App
            MenuAction::OpenSettings => {
                self.set_message("Settings (not yet implemented)");
            }
            MenuAction::Quit => {
                self.should_quit = true;
            }
        }
        Ok(())
    }

    /// Configure menu items based on current app state
    fn configure_menu_items(&self, controller: &mut MenuController) {
        let in_refactor = self.mode.is_refactor();
        let has_changes = self.pending_change_count() > 0;
        let is_paused = self.is_production_paused();

        // Production control items
        controller.set_enabled("start", is_paused);
        controller.set_enabled("pause", !is_paused && !in_refactor);

        // Refactor mode items
        controller.set_enabled("exit_refactor", in_refactor);
        controller.set_enabled("cancel_refactor", in_refactor);

        // Pending changes items
        controller.set_enabled("apply", has_changes);
        controller.set_enabled("discard", has_changes);
        controller.set_enabled("review", has_changes);

        // Update dynamic labels
        if has_changes {
            controller.update_label(
                "pending_ctrl",
                &format!("Pending Changes ({})", self.pending_change_count()),
            );
        }
    }

    /// Apply approved refactor changes
    pub async fn apply_refactor_changes(&mut self) -> Result<Vec<String>> {
        let agent = self
            .refactor_agent
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not in refactor mode"))?;

        let applied = agent.apply_changes(&self.project_root).await?;

        // Update state
        if let Some(session) = agent.session() {
            if let AppMode::Refactor(state) = &mut self.mode {
                state.update_from_session(session);
            }
        }

        Ok(applied)
    }

    /// Run the main TUI event loop.
    ///
    /// This handles:
    /// - Rendering the UI
    /// - Polling for keyboard events
    /// - Dispatching commands and input
    pub async fn run(&mut self, terminal: &mut Tui) -> Result<()> {
        // Input buffer for building commands/messages
        let mut input_buffer = String::new();

        // Menu controller for when menu is open
        let mut menu_controller: Option<MenuController> = None;

        while !self.should_quit {
            // If menu should open, create controller
            if self.menu_open && menu_controller.is_none() {
                let mut controller = MenuController::new(build_menu_tree());
                self.configure_menu_items(&mut controller);
                controller.open();
                menu_controller = Some(controller);
            }

            // Render the UI (with menu overlay if open)
            self.render_with_menu(terminal, &input_buffer, menu_controller.as_ref())?;

            // Poll for events with timeout (100ms for responsiveness)
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    // Ctrl+C always quits
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.should_quit = true;
                        continue;
                    }

                    // Handle menu input if menu is open
                    if let Some(ref mut controller) = menu_controller {
                        if let Some(action) = controller.handle_key(key.code) {
                            // Execute the action
                            self.execute_menu_action(action).await?;
                            // Close menu
                            self.menu_open = false;
                            menu_controller = None;
                        } else if !controller.is_open() {
                            // Menu was closed by Esc
                            self.menu_open = false;
                            menu_controller = None;
                        }
                        continue;
                    }

                    // Normal input handling
                    match key.code {
                        KeyCode::Enter => {
                            if !input_buffer.is_empty() {
                                let input = input_buffer.clone();
                                input_buffer.clear();
                                self.handle_input(&input).await?;
                            }
                        }
                        KeyCode::Backspace => {
                            input_buffer.pop();
                        }
                        KeyCode::Esc => {
                            if input_buffer.is_empty() {
                                // Open menu when Esc pressed with empty input
                                self.menu_open = true;
                            } else {
                                // Clear input
                                input_buffer.clear();
                                self.clear_message();
                            }
                        }
                        KeyCode::Char(c) => {
                            // Transition from Home to Main on first keypress
                            if self.view == View::Home {
                                self.view = View::Main;
                            }
                            input_buffer.push(c);
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    /// Render the UI with optional menu overlay
    fn render_with_menu(
        &self,
        terminal: &mut Tui,
        input_buffer: &str,
        menu_controller: Option<&MenuController>,
    ) -> io::Result<()> {
        terminal.draw(|frame| {
            let layout = AppLayout::new(frame.area());

            // Header bar with agent status
            let mode_indicator = self.mode.status_indicator();
            let header = HeaderBar::default().with_mode(&mode_indicator);
            let header = HeaderBar {
                current_view: format!("/{}", self.view.name().to_lowercase()),
                ..header
            };
            frame.render_widget(header, layout.header);

            // Main content area
            match self.view {
                View::Home => {
                    let home_view = if let Some(msg) = self.message() {
                        HomeView::default().with_message(msg)
                    } else {
                        HomeView::default()
                    };
                    frame.render_widget(home_view, layout.main);
                }
                View::Feed => {
                    let feed_view = FeedView {
                        items: self.feed.items.clone(),
                        scroll_offset: self.feed.scroll_offset,
                    };
                    frame.render_widget(feed_view, layout.main);
                }
                View::Main => {
                    let mut messages = self.chat.messages.clone();
                    if let Some(ref pending) = self.pending_response {
                        messages.push(pending.clone());
                    }
                    let chat_view = ChatView {
                        messages,
                        scroll_offset: self.chat.scroll_offset,
                    };
                    frame.render_widget(chat_view, layout.main);
                }
                View::Logs => {
                    let logs_view = LogsView {
                        entries: self.logs.entries.clone(),
                        scroll_offset: self.logs.scroll_offset,
                        min_level: self.logs.min_level,
                    };
                    frame.render_widget(logs_view, layout.main);
                }
                View::Tasks => {
                    let tasks_view = TasksView::default();
                    frame.render_widget(tasks_view, layout.main);
                }
                View::Agents => {
                    let agents_view = AgentsView::default();
                    frame.render_widget(agents_view, layout.main);
                }
                View::Costs => {
                    let costs_view = CostsView::default();
                    frame.render_widget(costs_view, layout.main);
                }
            }

            // Input bar
            let hint = if self.mode.is_refactor() {
                "Refactor mode - describe changes or /exit"
            } else {
                "Type /help for commands, Esc for menu"
            };
            let input_bar = InputBar::new(input_buffer).with_hint(hint);
            frame.render_widget(input_bar, layout.input);

            // Menu overlay (if open)
            if let Some(controller) = menu_controller {
                if let Some(menu) = controller.current_menu() {
                    let status = self.get_menu_status();
                    let (width, height) = menu_size(menu, &status);
                    let menu_area = centered_rect(width, height, frame.area());
                    let widget = MenuWidget::new(menu, controller.state(), &status);
                    frame.render_widget(widget, menu_area);
                }
            }
        })?;

        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_app() -> (App, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = crate::db::init_db_at(db_path.to_str().unwrap())
            .await
            .unwrap();
        let scheduler = Scheduler::new(pool.clone()).await.unwrap();
        let app = App::new(
            Arc::new(RwLock::new(scheduler)),
            pool,
            temp_dir.path().to_path_buf(),
        );
        (app, temp_dir)
    }

    #[tokio::test]
    async fn app_starts_in_normal_mode() {
        let (app, _temp_dir) = setup_app().await;
        assert!(app.mode().is_normal());
        assert_eq!(app.view(), View::Home);
    }

    #[tokio::test]
    async fn app_can_enter_refactor_mode() {
        let (mut app, _temp_dir) = setup_app().await;

        app.handle_input("/refactor").await.unwrap();

        assert!(app.mode().is_refactor());
        assert!(app.refactor_agent().is_some());
    }

    #[tokio::test]
    async fn app_can_exit_refactor_mode() {
        let (mut app, _temp_dir) = setup_app().await;

        app.handle_input("/refactor").await.unwrap();
        app.handle_input("/exit").await.unwrap();

        assert!(app.mode().is_normal());
        assert!(app.refactor_agent().is_none());
    }

    #[tokio::test]
    async fn app_handles_view_changes() {
        let (mut app, _temp_dir) = setup_app().await;

        app.handle_input("/feed").await.unwrap();
        assert_eq!(app.view(), View::Feed);

        app.handle_input("/main").await.unwrap();
        assert_eq!(app.view(), View::Main);

        app.handle_input("/logs").await.unwrap();
        assert_eq!(app.view(), View::Logs);
    }

    #[tokio::test]
    async fn app_handles_quit() {
        let (mut app, _temp_dir) = setup_app().await;

        assert!(!app.should_quit());
        app.handle_input("/quit").await.unwrap();
        assert!(app.should_quit());
    }

    #[tokio::test]
    async fn app_shows_help() {
        let (mut app, _temp_dir) = setup_app().await;

        app.handle_input("/help").await.unwrap();

        let msg = app.message().unwrap();
        assert!(msg.contains("/refactor"));
        assert!(msg.contains("/exit"));
    }

    #[tokio::test]
    async fn app_handles_unknown_command() {
        let (mut app, _temp_dir) = setup_app().await;

        app.handle_input("/foobar").await.unwrap();

        let msg = app.message().unwrap();
        assert!(msg.contains("Unknown command"));
    }

    #[tokio::test]
    async fn refactor_mode_processes_halt() {
        let (mut app, _temp_dir) = setup_app().await;

        app.handle_input("/refactor").await.unwrap();
        app.handle_input("STOP all work").await.unwrap();

        if let AppMode::Refactor(state) = app.mode() {
            assert!(state.production_halted);
        } else {
            panic!("Should be in refactor mode");
        }
    }

    #[tokio::test]
    async fn refactor_mode_can_exit_via_done() {
        let (mut app, _temp_dir) = setup_app().await;

        app.handle_input("/refactor").await.unwrap();
        app.handle_input("done").await.unwrap();

        assert!(app.mode().is_normal());
    }

    #[tokio::test]
    async fn status_bar_shows_mode() {
        let (mut app, _temp_dir) = setup_app().await;

        let normal_status = app.status_bar_text();
        assert!(normal_status.contains("Home"));
        assert!(!normal_status.contains("REFACTOR"));

        app.handle_input("/refactor").await.unwrap();
        let refactor_status = app.status_bar_text();
        assert!(refactor_status.contains("[REFACTOR]"));
    }

    #[tokio::test]
    async fn exit_in_normal_mode_quits() {
        let (mut app, _temp_dir) = setup_app().await;

        app.handle_input("/exit").await.unwrap();
        assert!(app.should_quit());
    }

    #[tokio::test]
    async fn double_refactor_shows_error() {
        let (mut app, _temp_dir) = setup_app().await;

        app.handle_input("/refactor").await.unwrap();
        app.handle_input("/refactor").await.unwrap();

        let msg = app.message().unwrap();
        assert!(msg.contains("Already in refactor mode"));
    }
}
