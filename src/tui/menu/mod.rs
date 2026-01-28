//! Menu system for the TUI
//!
//! Provides interactive menus for controlling production, refactor mode,
//! navigation, and other app functions.

mod builder;
mod controller;
mod types;
mod widget;

pub use builder::build_menu_tree;
pub use controller::MenuController;
pub use types::{Menu, MenuAction, MenuItem, MenuItemType, MenuState, MenuStatus};
pub use widget::{centered_rect, menu_size, MenuWidget};
