//! Terminal user interface
//!
//! This module provides:
//! - `App` - Main application state and logic
//! - `AppMode` - Operating modes (Normal, Refactor)
//! - `Command` - Slash command parsing and execution
//! - Views for different screens

mod app;
mod commands;
mod mode;
pub mod views;

pub use app::{App, View};
pub use commands::{Command, CommandResult};
pub use mode::{AppMode, RefactorModeState};
