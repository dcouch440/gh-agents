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
mod errors;
mod input;
mod layout;
pub mod menu;
mod mode;
pub mod views;

pub use app::{init_terminal, install_panic_hook, restore_terminal, App, Tui, View};
pub use commands::{Command, CommandResult};
pub use errors::{DisplayError, ErrorDisplay};
pub use input::InputBar;
pub use layout::{AppLayout, HeaderBar};
pub use menu::{
    build_menu_tree, centered_rect, menu_size, Menu, MenuAction, MenuController, MenuItem,
    MenuItemType, MenuState, MenuStatus, MenuWidget,
};
pub use mode::{AppMode, RefactorModeState};
