//! Command-line argument parsing for nexor

use clap::Parser;
use std::path::PathBuf;

/// AI Agent Orchestration for GitHub Workflows
#[derive(Parser, Debug)]
#[command(name = "nexor")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "3000")]
    pub port: u16,

    /// Override config file location
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Increase log verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl Args {
    /// Parse command-line arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Validate argument combinations
    pub fn validate(&self) -> Result<(), String> {
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
}

impl Default for Args {
    fn default() -> Self {
        Self {
            port: 3000,
            config: None,
            verbose: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid() {
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
