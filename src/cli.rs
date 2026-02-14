//! Command-line argument parsing for nexor

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// AI Agent Orchestration for GitHub Workflows
#[derive(Parser, Debug)]
#[command(name = "nexor")]
#[command(author, version, about, long_about = None)]
#[derive(Default)]
pub struct Args {
    /// Increase log verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Sync YAML configuration files to database
    SyncConfig {
        /// Directory containing config files
        #[arg(short = 'd', long, default_value = "./config")]
        config_dir: PathBuf,

        /// Dry run - validate configs without applying changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Start the server (default if no subcommand given)
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Override config file location
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
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

    /// Get port for server mode (default if no subcommand)
    pub fn port(&self) -> u16 {
        match &self.command {
            Some(Commands::Serve { port, .. }) => *port,
            _ => 3000,
        }
    }

    /// Get config file location
    pub fn config(&self) -> Option<PathBuf> {
        match &self.command {
            Some(Commands::Serve { config, .. }) => config.clone(),
            _ => None,
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
    fn default_values() {
        let args = Args::default();
        assert_eq!(args.port(), 3000);
        assert_eq!(args.config(), None);
        assert_eq!(args.verbose, 0);
    }

    #[test]
    fn log_level_increases_with_verbosity() {
        assert_eq!(
            Args {
                verbose: 0,
                ..Default::default()
            }
            .log_level(),
            tracing::Level::INFO
        );
        assert_eq!(
            Args {
                verbose: 1,
                ..Default::default()
            }
            .log_level(),
            tracing::Level::DEBUG
        );
        assert_eq!(
            Args {
                verbose: 2,
                ..Default::default()
            }
            .log_level(),
            tracing::Level::TRACE
        );
        assert_eq!(
            Args {
                verbose: 3,
                ..Default::default()
            }
            .log_level(),
            tracing::Level::TRACE
        );
    }

    #[test]
    fn serve_command_with_custom_port() {
        let args = Args {
            verbose: 0,
            command: Some(Commands::Serve {
                port: 8080,
                config: None,
            }),
        };
        assert_eq!(args.port(), 8080);
        assert!(args.validate().is_ok());
    }

    #[test]
    fn serve_command_with_config() {
        let config_path = PathBuf::from("/custom/config.toml");
        let args = Args {
            verbose: 0,
            command: Some(Commands::Serve {
                port: 3000,
                config: Some(config_path.clone()),
            }),
        };
        assert_eq!(args.config(), Some(config_path));
        assert!(args.validate().is_ok());
    }

    #[test]
    fn sync_config_command() {
        let args = Args {
            verbose: 1,
            command: Some(Commands::SyncConfig {
                config_dir: PathBuf::from("./config"),
                dry_run: false,
            }),
        };
        match args.command {
            Some(Commands::SyncConfig {
                config_dir,
                dry_run,
            }) => {
                assert_eq!(config_dir, PathBuf::from("./config"));
                assert_eq!(dry_run, false);
            }
            _ => panic!("Expected SyncConfig command"),
        }
    }

    #[test]
    fn debug_formatting() {
        let args = Args::default();
        let debug = format!("{:?}", args);
        assert!(debug.contains("Args"));
    }

    #[test]
    fn validate_always_succeeds() {
        let args1 = Args::default();
        assert!(args1.validate().is_ok());

        let args2 = Args {
            verbose: 255,
            command: None,
        };
        assert!(args2.validate().is_ok());
    }
}
