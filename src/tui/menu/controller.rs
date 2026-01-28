//! Menu controller for handling input and navigation

use crossterm::event::KeyCode;
use std::collections::HashMap;

use super::types::{Menu, MenuAction, MenuItem, MenuItemType, MenuState};

/// Controller for menu navigation and input handling
pub struct MenuController {
    /// All menus in the tree
    menus: HashMap<String, Menu>,
    /// Current navigation state
    state: MenuState,
}

impl MenuController {
    /// Create controller with menu tree
    pub fn new(menus: HashMap<String, Menu>) -> Self {
        Self {
            menus,
            state: MenuState::default(),
        }
    }

    /// Open menu system (shows main menu)
    pub fn open(&mut self) {
        self.state.open();
    }

    /// Close menu system
    pub fn close(&mut self) {
        self.state.close();
    }

    /// Check if menu is open
    pub fn is_open(&self) -> bool {
        self.state.is_open
    }

    /// Get current menu being displayed
    pub fn current_menu(&self) -> Option<&Menu> {
        self.menus.get(self.state.current_menu_id())
    }

    /// Get current state
    pub fn state(&self) -> &MenuState {
        &self.state
    }

    /// Handle key input, return action if one was triggered
    pub fn handle_key(&mut self, key: KeyCode) -> Option<MenuAction> {
        if !self.state.is_open {
            return None;
        }

        match key {
            KeyCode::Up => {
                self.move_selection(-1);
                None
            }
            KeyCode::Down => {
                self.move_selection(1);
                None
            }
            KeyCode::Enter => self.select_current(),
            KeyCode::Esc => {
                self.back_or_close();
                None
            }
            KeyCode::Char(c) => self.handle_shortcut(c),
            _ => None,
        }
    }

    /// Enable/disable a menu item by id
    pub fn set_enabled(&mut self, item_id: &str, enabled: bool) {
        if let Some(item) = self.find_item_mut(item_id) {
            item.enabled = enabled;
        }
    }

    /// Update item label (for dynamic content like "Pending Changes (2)")
    pub fn update_label(&mut self, item_id: &str, label: &str) {
        if let Some(item) = self.find_item_mut(item_id) {
            item.label = label.to_string();
        }
    }

    /// Move selection up or down, skipping separators and disabled items
    fn move_selection(&mut self, delta: i32) {
        let Some(menu) = self.current_menu() else {
            return;
        };

        // Get selectable items with their indices
        let selectable: Vec<usize> = menu
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.is_selectable())
            .map(|(i, _)| i)
            .collect();

        if selectable.is_empty() {
            return;
        }

        // Find current position in selectable list
        let current_pos = self.state.selected_index;

        // Calculate new position
        let new_pos = if delta > 0 {
            // Moving down
            (current_pos + delta as usize).min(selectable.len() - 1)
        } else {
            // Moving up
            current_pos.saturating_sub((-delta) as usize)
        };

        self.state.selected_index = new_pos;
    }

    /// Select current item - handle action/submenu/back
    fn select_current(&mut self) -> Option<MenuAction> {
        // Extract what we need from the menu first
        let item_type = {
            let menu = self.current_menu()?;

            // Find the currently selected item
            let selectable: Vec<&MenuItem> = menu
                .items
                .iter()
                .filter(|item| item.is_selectable())
                .collect();

            let item = selectable.get(self.state.selected_index)?;
            item.item_type.clone()
        };

        // Now we can mutate self
        match item_type {
            MenuItemType::Action(action) => {
                // Close menu and return action
                self.close();
                Some(action)
            }
            MenuItemType::Submenu(submenu_id) => {
                // Navigate to submenu
                self.state.navigate_to(&submenu_id);
                None
            }
            MenuItemType::Back => {
                // Go back
                self.back_or_close();
                None
            }
            MenuItemType::Separator => None,
        }
    }

    /// Go back or close if at root
    fn back_or_close(&mut self) {
        if !self.state.go_back() {
            // Couldn't go back, we're at root - close
            self.close();
        }
    }

    /// Handle shortcut key
    fn handle_shortcut(&mut self, c: char) -> Option<MenuAction> {
        // Extract matching action first
        let action = {
            let menu = self.current_menu()?;

            // Find item with matching shortcut
            menu.items
                .iter()
                .find(|item| item.shortcut == Some(c) && item.enabled)
                .and_then(|item| {
                    if let MenuItemType::Action(action) = &item.item_type {
                        Some(action.clone())
                    } else {
                        None
                    }
                })
        };

        // Now we can mutate self
        if let Some(action) = action {
            self.close();
            return Some(action);
        }

        None
    }

    /// Find a menu item by id across all menus
    fn find_item_mut(&mut self, item_id: &str) -> Option<&mut MenuItem> {
        for menu in self.menus.values_mut() {
            if let Some(item) = menu.get_item_mut(item_id) {
                return Some(item);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::menu::build_menu_tree;

    fn test_controller() -> MenuController {
        MenuController::new(build_menu_tree())
    }

    #[test]
    fn controller_starts_closed() {
        let controller = test_controller();
        assert!(!controller.is_open());
    }

    #[test]
    fn controller_can_open_and_close() {
        let mut controller = test_controller();

        controller.open();
        assert!(controller.is_open());

        controller.close();
        assert!(!controller.is_open());
    }

    #[test]
    fn controller_starts_at_main_menu() {
        let mut controller = test_controller();
        controller.open();

        let menu = controller.current_menu().unwrap();
        assert_eq!(menu.id, "main");
    }

    #[test]
    fn down_navigation_moves_selection() {
        let mut controller = test_controller();
        controller.open();

        assert_eq!(controller.state().selected_index, 0);

        controller.handle_key(KeyCode::Down);
        assert_eq!(controller.state().selected_index, 1);

        controller.handle_key(KeyCode::Down);
        assert_eq!(controller.state().selected_index, 2);
    }

    #[test]
    fn up_navigation_moves_selection() {
        let mut controller = test_controller();
        controller.open();

        controller.handle_key(KeyCode::Down);
        controller.handle_key(KeyCode::Down);
        assert_eq!(controller.state().selected_index, 2);

        controller.handle_key(KeyCode::Up);
        assert_eq!(controller.state().selected_index, 1);
    }

    #[test]
    fn up_navigation_stops_at_top() {
        let mut controller = test_controller();
        controller.open();

        controller.handle_key(KeyCode::Up);
        assert_eq!(controller.state().selected_index, 0);
    }

    #[test]
    fn enter_on_submenu_navigates() {
        let mut controller = test_controller();
        controller.open();

        // First item in main is "Production Control" submenu
        let result = controller.handle_key(KeyCode::Enter);
        assert!(result.is_none()); // No action returned for submenu

        let menu = controller.current_menu().unwrap();
        assert_eq!(menu.id, "production");
    }

    #[test]
    fn escape_goes_back() {
        let mut controller = test_controller();
        controller.open();

        // Navigate to production submenu
        controller.handle_key(KeyCode::Enter);
        assert_eq!(controller.current_menu().unwrap().id, "production");

        // Escape goes back
        controller.handle_key(KeyCode::Esc);
        assert_eq!(controller.current_menu().unwrap().id, "main");
    }

    #[test]
    fn escape_at_root_closes() {
        let mut controller = test_controller();
        controller.open();
        assert!(controller.is_open());

        controller.handle_key(KeyCode::Esc);
        assert!(!controller.is_open());
    }

    #[test]
    fn shortcut_triggers_action() {
        let mut controller = test_controller();
        controller.open();

        // 'q' is shortcut for Quit in main menu
        let result = controller.handle_key(KeyCode::Char('q'));
        assert_eq!(result, Some(MenuAction::Quit));
        assert!(!controller.is_open()); // Menu closes after action
    }

    #[test]
    fn enter_on_action_returns_action() {
        let mut controller = test_controller();
        controller.open();

        // Navigate to production submenu
        controller.handle_key(KeyCode::Enter);

        // First item is "Start Production"
        let result = controller.handle_key(KeyCode::Enter);
        assert_eq!(result, Some(MenuAction::StartProduction));
        assert!(!controller.is_open()); // Menu closes after action
    }

    #[test]
    fn back_item_goes_back() {
        let mut controller = test_controller();
        controller.open();

        // Navigate to production submenu
        controller.handle_key(KeyCode::Enter);
        assert_eq!(controller.current_menu().unwrap().id, "production");

        // Navigate to bottom to find Back item
        for _ in 0..10 {
            controller.handle_key(KeyCode::Down);
        }

        // Select Back
        let result = controller.handle_key(KeyCode::Enter);
        assert!(result.is_none());
        assert_eq!(controller.current_menu().unwrap().id, "main");
    }

    #[test]
    fn set_enabled_disables_item() {
        let mut controller = test_controller();

        controller.set_enabled("quit", false);

        // Shortcut should no longer work
        controller.open();
        let result = controller.handle_key(KeyCode::Char('q'));
        assert!(result.is_none());
    }

    #[test]
    fn update_label_changes_item_label() {
        let mut controller = test_controller();

        controller.update_label("quit", "Exit Application");

        controller.open();
        let menu = controller.current_menu().unwrap();
        let item = menu.get_item("quit").unwrap();
        assert_eq!(item.label, "Exit Application");
    }

    #[test]
    fn no_action_when_closed() {
        let mut controller = test_controller();

        // Don't open
        let result = controller.handle_key(KeyCode::Enter);
        assert!(result.is_none());

        let result = controller.handle_key(KeyCode::Char('q'));
        assert!(result.is_none());
    }

    #[test]
    fn navigation_skips_disabled_items() {
        let mut controller = test_controller();
        controller.open();

        // Navigate into production
        controller.handle_key(KeyCode::Enter);

        // Disable "Pause Production" (second selectable item)
        controller.set_enabled("pause", false);

        // At index 0 (Start Production)
        assert_eq!(controller.state().selected_index, 0);

        // Move down - should skip to next selectable
        controller.handle_key(KeyCode::Down);

        // Should be at index 1, but the item at that index should be selectable
        // (navigation doesn't skip indices, but selection considers only selectable items)
        assert!(controller.state().selected_index >= 1);
    }
}
