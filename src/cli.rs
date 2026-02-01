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
    fn default_values() {
        let args = Args::default();
        assert_eq!(args.port, 3000);
        assert_eq!(args.config, None);
        assert_eq!(args.verbose, 0);
    }

    #[test]
    fn log_level_increases_with_verbosity() {
        assert_eq!(Args { verbose: 0, ..Default::default() }.log_level(), tracing::Level::INFO);
        assert_eq!(Args { verbose: 1, ..Default::default() }.log_level(), tracing::Level::DEBUG);
        assert_eq!(Args { verbose: 2, ..Default::default() }.log_level(), tracing::Level::TRACE);
        assert_eq!(Args { verbose: 3, ..Default::default() }.log_level(), tracing::Level::TRACE);
    }

    #[test]
    fn custom_port() {
        let args = Args {
            port: 8080,
            ..Default::default()
        };
        assert_eq!(args.port, 8080);
        assert!(args.validate().is_ok());
    }

    #[test]
    fn custom_config_path() {
        let config_path = PathBuf::from("/custom/config.toml");
        let args = Args {
            config: Some(config_path.clone()),
            ..Default::default()
        };
        assert_eq!(args.config, Some(config_path));
        assert!(args.validate().is_ok());
    }

    #[test]
    fn all_options_together() {
        let config_path = PathBuf::from("./config.toml");
        let args = Args {
            port: 9000,
            config: Some(config_path.clone()),
            verbose: 2,
        };

        assert_eq!(args.port, 9000);
        assert_eq!(args.config, Some(config_path));
        assert_eq!(args.verbose, 2);
        assert_eq!(args.log_level(), tracing::Level::TRACE);
        assert!(args.validate().is_ok());
    }

    #[test]
    fn debug_formatting() {
        let args = Args::default();
        let debug = format!("{:?}", args);
        assert!(debug.contains("Args"));
        assert!(debug.contains("3000"));
    }

    #[test]
    fn validate_always_succeeds() {
        // Since validate() currently always returns Ok(()), test various configurations
        let args1 = Args::default();
        assert!(args1.validate().is_ok());

        let args2 = Args {
            port: 0,
            config: None,
            verbose: 255,
        };
        assert!(args2.validate().is_ok());

        let args3 = Args {
            port: 65535,
            config: Some(PathBuf::from("/nonexistent/path")),
            verbose: 10,
        };
        assert!(args3.validate().is_ok());
    }

    #[test]
    fn log_level_info_at_zero_verbosity() {
        let args = Args {
            verbose: 0,
            ..Default::default()
        };
        assert_eq!(args.log_level(), tracing::Level::INFO);
    }

    #[test]
    fn log_level_debug_at_one_verbosity() {
        let args = Args {
            verbose: 1,
            ..Default::default()
        };
        assert_eq!(args.log_level(), tracing::Level::DEBUG);
    }

    #[test]
    fn log_level_trace_at_high_verbosity() {
        let args = Args {
            verbose: 100,
            ..Default::default()
        };
        assert_eq!(args.log_level(), tracing::Level::TRACE);
    }

    #[test]
    fn config_path_can_be_none() {
        let args = Args {
            config: None,
            ..Default::default()
        };
        assert!(args.config.is_none());
    }

    #[test]
    fn config_path_can_be_relative() {
        let args = Args {
            config: Some(PathBuf::from("config.toml")),
            ..Default::default()
        };
        assert_eq!(args.config.unwrap().to_str(), Some("config.toml"));
    }

    #[test]
    fn config_path_can_be_absolute() {
        let args = Args {
            config: Some(PathBuf::from("/etc/nexor/config.toml")),
            ..Default::default()
        };
        assert_eq!(args.config.unwrap().to_str(), Some("/etc/nexor/config.toml"));
    }

    #[test]
    fn port_range_values() {
        // Test common port values
        let args_min = Args {
            port: 1,
            ..Default::default()
        };
        assert_eq!(args_min.port, 1);

        let args_standard = Args {
            port: 3000,
            ..Default::default()
        };
        assert_eq!(args_standard.port, 3000);

        let args_max = Args {
            port: 65535,
            ..Default::default()
        };
        assert_eq!(args_max.port, 65535);
    }
}
