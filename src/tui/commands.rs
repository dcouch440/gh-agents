//! Slash command handling for the TUI.
//!
//! Commands are entered with a leading `/` and can:
//! - Switch modes (/refactor, /exit)
//! - Navigate views (/feed, /main, /logs, /tasks, /agents, /costs)
//! - Perform actions (/help, /quit)

use std::fmt;

/// A parsed slash command
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Enter refactor mode
    Refactor,
    /// Exit current mode (or quit if in normal mode)
    Exit,
    /// Show help
    Help,
    /// Quit the application
    Quit,
    /// Show the feed view
    Feed,
    /// Show the main chat view
    Main,
    /// Show the logs view
    Logs,
    /// Show the tasks view
    Tasks,
    /// Show the agents view
    Agents,
    /// Show the costs view
    Costs,
    /// Unknown command
    Unknown(String),
}

impl Command {
    /// Parse a command from input string.
    ///
    /// Returns `None` if the input doesn't start with `/`.
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        if !input.starts_with('/') {
            return None;
        }

        let parts: Vec<&str> = input[1..].split_whitespace().collect();
        let cmd = parts.first().map(|s| s.to_lowercase())?;

        Some(match cmd.as_str() {
            "refactor" => Command::Refactor,
            "exit" | "done" => Command::Exit,
            "help" | "?" => Command::Help,
            "quit" | "q" => Command::Quit,
            "feed" => Command::Feed,
            "main" | "chat" => Command::Main,
            "logs" => Command::Logs,
            "tasks" => Command::Tasks,
            "agents" => Command::Agents,
            "costs" => Command::Costs,
            other => Command::Unknown(other.to_string()),
        })
    }

    /// Get the command name (without the leading /)
    pub fn name(&self) -> &str {
        match self {
            Command::Refactor => "refactor",
            Command::Exit => "exit",
            Command::Help => "help",
            Command::Quit => "quit",
            Command::Feed => "feed",
            Command::Main => "main",
            Command::Logs => "logs",
            Command::Tasks => "tasks",
            Command::Agents => "agents",
            Command::Costs => "costs",
            Command::Unknown(name) => name,
        }
    }

    /// Get a short description of the command
    pub fn description(&self) -> &str {
        match self {
            Command::Refactor => "Enter refactor mode to modify the project plan",
            Command::Exit => "Exit current mode or quit",
            Command::Help => "Show available commands",
            Command::Quit => "Quit the application",
            Command::Feed => "Show the activity feed",
            Command::Main => "Show the main chat view",
            Command::Logs => "Show application logs",
            Command::Tasks => "Show task list",
            Command::Agents => "Show agent status",
            Command::Costs => "Show cost breakdown",
            Command::Unknown(_) => "Unknown command",
        }
    }

    /// Check if this command is valid
    pub fn is_valid(&self) -> bool {
        !matches!(self, Command::Unknown(_))
    }

    /// Get all available commands with descriptions
    pub fn all_commands() -> Vec<(&'static str, &'static str)> {
        vec![
            ("/refactor", "Enter refactor mode to modify the project plan"),
            ("/exit", "Exit current mode or quit"),
            ("/help", "Show available commands"),
            ("/quit", "Quit the application"),
            ("/feed", "Show the activity feed"),
            ("/main", "Show the main chat view"),
            ("/logs", "Show application logs"),
            ("/tasks", "Show task list"),
            ("/agents", "Show agent status"),
            ("/costs", "Show cost breakdown"),
        ]
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/{}", self.name())
    }
}

/// Result of executing a command
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Command executed successfully with a message
    Success(String),
    /// Command failed with an error message
    Error(String),
    /// Mode changed
    ModeChanged,
    /// View changed
    ViewChanged,
    /// Should quit the application
    Quit,
    /// Show help text
    Help(String),
}

/// Generate the help text for all commands
pub fn generate_help_text() -> String {
    let mut help = String::from("Available commands:\n\n");

    for (cmd, desc) in Command::all_commands() {
        help.push_str(&format!("  {:<12} {}\n", cmd, desc));
    }

    help.push_str("\nIn refactor mode:\n");
    help.push_str("  - Chat naturally to describe changes you want to make\n");
    help.push_str("  - Say \"STOP\" or \"halt\" to pause production\n");
    help.push_str("  - Say \"done\" or \"/exit\" to leave refactor mode\n");

    help
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_known_commands() {
        assert_eq!(Command::parse("/refactor"), Some(Command::Refactor));
        assert_eq!(Command::parse("/exit"), Some(Command::Exit));
        assert_eq!(Command::parse("/done"), Some(Command::Exit));
        assert_eq!(Command::parse("/help"), Some(Command::Help));
        assert_eq!(Command::parse("/?"), Some(Command::Help));
        assert_eq!(Command::parse("/quit"), Some(Command::Quit));
        assert_eq!(Command::parse("/q"), Some(Command::Quit));
        assert_eq!(Command::parse("/feed"), Some(Command::Feed));
        assert_eq!(Command::parse("/main"), Some(Command::Main));
        assert_eq!(Command::parse("/chat"), Some(Command::Main));
        assert_eq!(Command::parse("/logs"), Some(Command::Logs));
        assert_eq!(Command::parse("/tasks"), Some(Command::Tasks));
        assert_eq!(Command::parse("/agents"), Some(Command::Agents));
        assert_eq!(Command::parse("/costs"), Some(Command::Costs));
    }

    #[test]
    fn parse_unknown_command() {
        assert_eq!(
            Command::parse("/foo"),
            Some(Command::Unknown("foo".to_string()))
        );
    }

    #[test]
    fn parse_not_a_command() {
        assert_eq!(Command::parse("hello"), None);
        assert_eq!(Command::parse(""), None);
        assert_eq!(Command::parse("  "), None);
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!(Command::parse("/REFACTOR"), Some(Command::Refactor));
        assert_eq!(Command::parse("/Refactor"), Some(Command::Refactor));
    }

    #[test]
    fn parse_with_whitespace() {
        assert_eq!(Command::parse("  /refactor  "), Some(Command::Refactor));
    }

    #[test]
    fn command_display() {
        assert_eq!(format!("{}", Command::Refactor), "/refactor");
        assert_eq!(format!("{}", Command::Exit), "/exit");
    }

    #[test]
    fn command_validity() {
        assert!(Command::Refactor.is_valid());
        assert!(Command::Exit.is_valid());
        assert!(!Command::Unknown("foo".to_string()).is_valid());
    }

    #[test]
    fn help_text_contains_all_commands() {
        let help = generate_help_text();
        for (cmd, _) in Command::all_commands() {
            assert!(help.contains(cmd), "Help text should contain {}", cmd);
        }
    }
}
