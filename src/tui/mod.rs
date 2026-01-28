//! Terminal user interface
//!
//! This module provides:
//! - `App` - Main application state and logic
//! - `AppMode` - Operating modes (Normal, Refactor)
//! - `Command` - Slash command parsing and execution
//! - `AppLayout`, `HeaderBar` - Layout system
//! - `InputBar` - Input widget
//! - Views for different screens

mod app;
mod commands;
mod input;
mod layout;
mod mode;
pub mod views;

pub use app::{init_terminal, install_panic_hook, restore_terminal, App, Tui, View};
pub use commands::{Command, CommandResult};
pub use input::InputBar;
pub use layout::{AppLayout, HeaderBar};
pub use mode::{AppMode, RefactorModeState};
