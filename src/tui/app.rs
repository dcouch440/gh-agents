//! Main TUI application state and logic.
//!
//! The App struct manages:
//! - Current mode (Normal, Refactor)
//! - Current view (Feed, Main, Logs, etc.)
//! - User input processing
//! - Command routing

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::orchestration::Scheduler;
use crate::refactor::RefactorAgent;
use crate::tui::commands::{generate_help_text, Command, CommandResult};
use crate::tui::mode::{AppMode, RefactorModeState};

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
}

impl App {
    /// Create a new application
    pub fn new(
        scheduler: Arc<RwLock<Scheduler>>,
        pool: sqlx::SqlitePool,
        project_root: PathBuf,
    ) -> Self {
        Self {
            mode: AppMode::Normal,
            view: View::Home,
            scheduler,
            pool,
            refactor_agent: None,
            project_root,
            should_quit: false,
            message: None,
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
            format!("{} {} | {}{}", mode_indicator, view_name, "Type /exit to leave", state_summary)
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

        // In normal mode, non-command input goes to the orchestrator
        // (To be implemented when orchestrator is complete)
        self.set_message(format!("Input: {}", input));
        Ok(true)
    }

    /// Execute a slash command
    async fn execute_command(&mut self, cmd: Command) -> Result<CommandResult> {
        match cmd {
            Command::Refactor => self.enter_refactor_mode().await,
            Command::Exit => self.handle_exit().await,
            Command::Help => Ok(CommandResult::Help(generate_help_text())),
            Command::Quit => Ok(CommandResult::Quit),
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
            AppMode::Refactor(_) => {
                self.exit_refactor_mode().await
            }
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
