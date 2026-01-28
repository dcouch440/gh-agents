//! Menu system types for interactive TUI menus

use crate::tui::View;

/// A menu item that can be selected
#[derive(Debug, Clone)]
pub struct MenuItem {
    /// Unique identifier for this item
    pub id: String,
    /// Display label
    pub label: String,
    /// Optional keyboard shortcut
    pub shortcut: Option<char>,
    /// What type of menu item this is
    pub item_type: MenuItemType,
    /// Whether the item is currently enabled
    pub enabled: bool,
}

impl MenuItem {
    /// Create a new menu item
    pub fn new(id: impl Into<String>, label: impl Into<String>, item_type: MenuItemType) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            item_type,
            enabled: true,
        }
    }

    /// Create an action item
    pub fn action(id: impl Into<String>, label: impl Into<String>, action: MenuAction) -> Self {
        Self::new(id, label, MenuItemType::Action(action))
    }

    /// Create a submenu item
    pub fn submenu(id: impl Into<String>, label: impl Into<String>, submenu_id: impl Into<String>) -> Self {
        Self::new(id, label, MenuItemType::Submenu(submenu_id.into()))
    }

    /// Create a separator
    pub fn separator() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            shortcut: None,
            item_type: MenuItemType::Separator,
            enabled: false,
        }
    }

    /// Create a back item
    pub fn back() -> Self {
        Self::new("back", "Back", MenuItemType::Back)
    }

    /// Set keyboard shortcut
    pub fn with_shortcut(mut self, shortcut: char) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Check if this item is selectable
    pub fn is_selectable(&self) -> bool {
        self.enabled && !matches!(self.item_type, MenuItemType::Separator)
    }
}

/// Type of menu item
#[derive(Debug, Clone)]
pub enum MenuItemType {
    /// Triggers an action when selected
    Action(MenuAction),
    /// Opens a submenu when selected
    Submenu(String),
    /// Visual separator (not selectable)
    Separator,
    /// Go back to previous menu
    Back,
}

/// Actions that can be triggered by menu items
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    // Production
    /// Start production mode
    StartProduction,
    /// Pause production mode
    PauseProduction,
    /// Set milestone limit (1-9)
    SetMilestoneLimit(u8),
    /// Clear milestone limit
    ClearMilestoneLimit,

    // Refactor
    /// Exit refactor mode
    ExitRefactorMode,
    /// Cancel current refactor
    CancelRefactor,
    /// Apply pending changes
    ApplyChanges,
    /// Discard pending changes
    DiscardChanges,
    /// Review pending changes
    ReviewChanges,

    // Navigation
    /// Navigate to a view
    GoToView(View),

    // App
    /// Open settings
    OpenSettings,
    /// Quit the application
    Quit,
}

/// A complete menu definition
#[derive(Debug, Clone)]
pub struct Menu {
    /// Unique identifier for this menu
    pub id: String,
    /// Display title
    pub title: String,
    /// Menu items
    pub items: Vec<MenuItem>,
}

impl Menu {
    /// Create a new menu
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            items: Vec::new(),
        }
    }

    /// Add an item to the menu
    pub fn add_item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add multiple items
    pub fn add_items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Get the number of selectable items
    pub fn selectable_count(&self) -> usize {
        self.items.iter().filter(|i| i.is_selectable()).count()
    }

    /// Get item by id
    pub fn get_item(&self, id: &str) -> Option<&MenuItem> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Get mutable item by id
    pub fn get_item_mut(&mut self, id: &str) -> Option<&mut MenuItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }
}

/// State of the menu during interaction
#[derive(Debug, Clone)]
pub struct MenuState {
    /// Stack of menu IDs for navigation (first is root, last is current)
    pub stack: Vec<String>,
    /// Currently selected item index
    pub selected_index: usize,
    /// Whether the menu is open
    pub is_open: bool,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            stack: vec!["main".to_string()],
            selected_index: 0,
            is_open: false,
        }
    }
}

impl MenuState {
    /// Create a new menu state
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the menu
    pub fn open(&mut self) {
        self.is_open = true;
        self.stack = vec!["main".to_string()];
        self.selected_index = 0;
    }

    /// Close the menu
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Get the current menu ID
    pub fn current_menu_id(&self) -> &str {
        self.stack.last().map(|s| s.as_str()).unwrap_or("main")
    }

    /// Navigate to a submenu
    pub fn navigate_to(&mut self, menu_id: impl Into<String>) {
        self.stack.push(menu_id.into());
        self.selected_index = 0;
    }

    /// Go back to previous menu
    pub fn go_back(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            self.selected_index = 0;
            true
        } else {
            false
        }
    }

    /// Select next item
    pub fn select_next(&mut self, max_items: usize) {
        if max_items > 0 && self.selected_index < max_items - 1 {
            self.selected_index += 1;
        }
    }

    /// Select previous item
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Check if at root menu
    pub fn is_at_root(&self) -> bool {
        self.stack.len() <= 1
    }
}

/// Status info for display in menu header
#[derive(Debug, Clone, Default)]
pub struct MenuStatus {
    /// Current production state description
    pub production_state: String,
    /// Current milestone limit (e.g., "M3" or "None")
    pub current_milestone: String,
    /// Number of pending changes
    pub pending_changes: usize,
}

impl MenuStatus {
    /// Create new menu status
    pub fn new() -> Self {
        Self::default()
    }

    /// Set production state
    pub fn with_production_state(mut self, state: impl Into<String>) -> Self {
        self.production_state = state.into();
        self
    }

    /// Set milestone limit
    pub fn with_milestone(mut self, milestone: impl Into<String>) -> Self {
        self.current_milestone = milestone.into();
        self
    }

    /// Set pending changes count
    pub fn with_pending_changes(mut self, count: usize) -> Self {
        self.pending_changes = count;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_item_action() {
        let item = MenuItem::action("start", "Start Production", MenuAction::StartProduction);
        assert_eq!(item.id, "start");
        assert_eq!(item.label, "Start Production");
        assert!(matches!(item.item_type, MenuItemType::Action(MenuAction::StartProduction)));
        assert!(item.enabled);
    }

    #[test]
    fn menu_item_submenu() {
        let item = MenuItem::submenu("prod", "Production Control", "production");
        assert!(matches!(item.item_type, MenuItemType::Submenu(ref id) if id == "production"));
    }

    #[test]
    fn menu_item_separator() {
        let item = MenuItem::separator();
        assert!(!item.is_selectable());
    }

    #[test]
    fn menu_item_back() {
        let item = MenuItem::back();
        assert_eq!(item.id, "back");
        assert!(matches!(item.item_type, MenuItemType::Back));
    }

    #[test]
    fn menu_item_shortcut() {
        let item = MenuItem::action("quit", "Quit", MenuAction::Quit)
            .with_shortcut('q');
        assert_eq!(item.shortcut, Some('q'));
    }

    #[test]
    fn menu_item_enabled() {
        let item = MenuItem::action("test", "Test", MenuAction::Quit)
            .with_enabled(false);
        assert!(!item.enabled);
        assert!(!item.is_selectable());
    }

    #[test]
    fn menu_builder() {
        let menu = Menu::new("test", "Test Menu")
            .add_item(MenuItem::action("a", "Item A", MenuAction::Quit))
            .add_item(MenuItem::separator())
            .add_item(MenuItem::action("b", "Item B", MenuAction::Quit));

        assert_eq!(menu.id, "test");
        assert_eq!(menu.title, "Test Menu");
        assert_eq!(menu.items.len(), 3);
        assert_eq!(menu.selectable_count(), 2);
    }

    #[test]
    fn menu_get_item() {
        let menu = Menu::new("test", "Test")
            .add_item(MenuItem::action("a", "A", MenuAction::Quit))
            .add_item(MenuItem::action("b", "B", MenuAction::Quit));

        assert!(menu.get_item("a").is_some());
        assert!(menu.get_item("c").is_none());
    }

    #[test]
    fn menu_state_default() {
        let state = MenuState::default();
        assert!(!state.is_open);
        assert_eq!(state.current_menu_id(), "main");
        assert!(state.is_at_root());
    }

    #[test]
    fn menu_state_open_close() {
        let mut state = MenuState::new();
        assert!(!state.is_open);

        state.open();
        assert!(state.is_open);

        state.close();
        assert!(!state.is_open);
    }

    #[test]
    fn menu_state_navigation() {
        let mut state = MenuState::new();
        state.open();

        state.navigate_to("production");
        assert_eq!(state.current_menu_id(), "production");
        assert!(!state.is_at_root());

        state.navigate_to("milestone");
        assert_eq!(state.current_menu_id(), "milestone");
        assert_eq!(state.stack.len(), 3);

        assert!(state.go_back());
        assert_eq!(state.current_menu_id(), "production");

        assert!(state.go_back());
        assert_eq!(state.current_menu_id(), "main");

        assert!(!state.go_back()); // Can't go back from root
    }

    #[test]
    fn menu_state_selection() {
        let mut state = MenuState::new();

        state.select_next(5);
        assert_eq!(state.selected_index, 1);

        state.select_next(5);
        state.select_next(5);
        state.select_next(5);
        assert_eq!(state.selected_index, 4);

        state.select_next(5); // Can't go past max
        assert_eq!(state.selected_index, 4);

        state.select_prev();
        assert_eq!(state.selected_index, 3);

        state.selected_index = 0;
        state.select_prev(); // Can't go below 0
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn menu_status_builder() {
        let status = MenuStatus::new()
            .with_production_state("Running")
            .with_milestone("M3")
            .with_pending_changes(5);

        assert_eq!(status.production_state, "Running");
        assert_eq!(status.current_milestone, "M3");
        assert_eq!(status.pending_changes, 5);
    }
}
