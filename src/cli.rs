//! Command-line argument parsing for nexor

use clap::Parser;
use std::path::PathBuf;

/// AI Agent Orchestration TUI for GitHub Workflows
#[derive(Parser, Debug)]
#[command(name = "nexor")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Run in headless mode (no TUI)
    #[arg(short = 'H', long)]
    pub headless: bool,

    /// Task description to process (headless mode)
    #[arg(short, long)]
    pub task: Option<String>,

    /// Read task(s) from file (headless mode)
    #[arg(short, long)]
    pub input: Option<PathBuf>,

    /// Write output to file instead of stdout (headless mode)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Override config file location
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Increase log verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// GitHub issue URL to sync and work on
    #[arg(long)]
    pub sync: Option<String>,
}

impl Args {
    /// Parse command-line arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Validate argument combinations
    pub fn validate(&self) -> Result<(), String> {
        if self.headless {
            // In headless mode, need either --task, --input, or --sync
            if self.task.is_none() && self.input.is_none() && self.sync.is_none() {
                return Err(
                    "headless mode requires --task, --input, or --sync".to_string()
                );
            }
        }

        if self.input.is_some() && !self.headless {
            return Err("--input requires --headless mode".to_string());
        }

        if self.output.is_some() && !self.headless {
            return Err("--output requires --headless mode".to_string());
        }

        Ok(())
    }

    /// Get the tracing log level based on verbosity
    pub fn log_level(&self) -> tracing::Level {
        match self.verbose {
            0 => tracing::Level::INFO,
            1 => tracing::Level::DEBUG,
            _ => tracing::Level::TRACE,
        }
    }

    /// Check if running in headless mode
    pub fn is_headless(&self) -> bool {
        self.headless
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            headless: false,
            task: None,
            input: None,
            output: None,
            config: None,
            verbose: 0,
            sync: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_requires_task_or_input() {
        let args = Args {
            headless: true,
            task: None,
            input: None,
            sync: None,
            ..Default::default()
        };
        assert!(args.validate().is_err());
    }

    #[test]
    fn headless_with_task_valid() {
        let args = Args {
            headless: true,
            task: Some("test task".to_string()),
            ..Default::default()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn headless_with_input_valid() {
        let args = Args {
            headless: true,
            input: Some(PathBuf::from("tasks.txt")),
            ..Default::default()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn headless_with_sync_valid() {
        let args = Args {
            headless: true,
            sync: Some("https://github.com/owner/repo/issues/1".to_string()),
            ..Default::default()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn input_requires_headless() {
        let args = Args {
            headless: false,
            input: Some(PathBuf::from("tasks.txt")),
            ..Default::default()
        };
        assert!(args.validate().is_err());
    }

    #[test]
    fn output_requires_headless() {
        let args = Args {
            headless: false,
            output: Some(PathBuf::from("output.txt")),
            ..Default::default()
        };
        assert!(args.validate().is_err());
    }

    #[test]
    fn non_headless_is_valid() {
        let args = Args::default();
        assert!(args.validate().is_ok());
    }

    #[test]
    fn log_level_increases_with_verbosity() {
        assert_eq!(Args { verbose: 0, ..Default::default() }.log_level(), tracing::Level::INFO);
        assert_eq!(Args { verbose: 1, ..Default::default() }.log_level(), tracing::Level::DEBUG);
        assert_eq!(Args { verbose: 2, ..Default::default() }.log_level(), tracing::Level::TRACE);
        assert_eq!(Args { verbose: 3, ..Default::default() }.log_level(), tracing::Level::TRACE);
    }
}
