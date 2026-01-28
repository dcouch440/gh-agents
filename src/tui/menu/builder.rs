//! Menu builder for constructing the menu tree

use super::types::{Menu, MenuAction, MenuItem};
use crate::tui::View;
use std::collections::HashMap;

/// Build the complete menu tree
pub fn build_menu_tree() -> HashMap<String, Menu> {
    let mut menus = HashMap::new();

    menus.insert("main".to_string(), build_main_menu());
    menus.insert("production".to_string(), build_production_control_menu());
    menus.insert("milestone".to_string(), build_milestone_limit_menu());
    menus.insert("refactor".to_string(), build_refactor_mode_menu());
    menus.insert("pending".to_string(), build_pending_changes_menu());
    menus.insert("navigate".to_string(), build_navigate_menu());

    menus
}

/// Build the main menu
fn build_main_menu() -> Menu {
    Menu::new("main", "Menu")
        .add_item(MenuItem::submenu("prod_ctrl", "Production Control", "production"))
        .add_item(MenuItem::submenu("refactor_ctrl", "Refactor Mode", "refactor"))
        .add_item(MenuItem::submenu("pending_ctrl", "Pending Changes", "pending"))
        .add_item(MenuItem::submenu("nav", "Navigate", "navigate"))
        .add_item(MenuItem::separator())
        .add_item(MenuItem::action("settings", "Settings", MenuAction::OpenSettings))
        .add_item(MenuItem::separator())
        .add_item(
            MenuItem::action("quit", "Quit", MenuAction::Quit)
                .with_shortcut('q'),
        )
}

/// Build the production control submenu
fn build_production_control_menu() -> Menu {
    Menu::new("production", "Production Control")
        .add_item(MenuItem::action("start", "Start Production", MenuAction::StartProduction))
        .add_item(MenuItem::action("pause", "Pause Production", MenuAction::PauseProduction))
        .add_item(MenuItem::separator())
        .add_item(MenuItem::submenu("milestone_ctrl", "Set Milestone Limit", "milestone"))
        .add_item(MenuItem::action(
            "clear_limit",
            "Clear Milestone Limit",
            MenuAction::ClearMilestoneLimit,
        ))
        .add_item(MenuItem::separator())
        .add_item(MenuItem::back())
}

/// Build the milestone limit submenu
fn build_milestone_limit_menu() -> Menu {
    let mut menu = Menu::new("milestone", "Set Milestone Limit");

    // Add milestone options M1-M9
    let milestones = [
        (1, "M1: Foundation"),
        (2, "M2: LLM Layer"),
        (3, "M3: Agent Runtime"),
        (4, "M4: Prompt Engineering"),
        (5, "M5: Orchestration Core"),
        (6, "M6: TUI Basic"),
        (7, "M7: Execution Layer"),
        (8, "M8: GitHub Integration"),
        (9, "M9: Polish & Production"),
    ];

    for (num, label) in milestones {
        menu = menu.add_item(MenuItem::action(
            format!("m{}", num),
            label,
            MenuAction::SetMilestoneLimit(num),
        ));
    }

    menu.add_item(MenuItem::separator())
        .add_item(MenuItem::back())
}

/// Build the refactor mode submenu
fn build_refactor_mode_menu() -> Menu {
    Menu::new("refactor", "Refactor Mode")
        .add_item(MenuItem::action(
            "exit_refactor",
            "Exit Refactor Mode",
            MenuAction::ExitRefactorMode,
        ))
        .add_item(MenuItem::action(
            "cancel_refactor",
            "Cancel Refactor",
            MenuAction::CancelRefactor,
        ))
        .add_item(MenuItem::separator())
        .add_item(MenuItem::back())
}

/// Build the pending changes submenu
fn build_pending_changes_menu() -> Menu {
    Menu::new("pending", "Pending Changes")
        .add_item(MenuItem::action("apply", "Apply Changes", MenuAction::ApplyChanges))
        .add_item(MenuItem::action("discard", "Discard Changes", MenuAction::DiscardChanges))
        .add_item(MenuItem::action("review", "Review Changes", MenuAction::ReviewChanges))
        .add_item(MenuItem::separator())
        .add_item(MenuItem::back())
}

/// Build the navigate submenu
fn build_navigate_menu() -> Menu {
    Menu::new("navigate", "Navigate")
        .add_item(MenuItem::action("nav_home", "Home", MenuAction::GoToView(View::Home)))
        .add_item(MenuItem::action("nav_feed", "Feed", MenuAction::GoToView(View::Feed)))
        .add_item(MenuItem::action("nav_main", "Main Chat", MenuAction::GoToView(View::Main)))
        .add_item(MenuItem::action("nav_logs", "Logs", MenuAction::GoToView(View::Logs)))
        .add_item(MenuItem::action("nav_tasks", "Tasks", MenuAction::GoToView(View::Tasks)))
        .add_item(MenuItem::action("nav_agents", "Agents", MenuAction::GoToView(View::Agents)))
        .add_item(MenuItem::action("nav_costs", "Costs", MenuAction::GoToView(View::Costs)))
        .add_item(MenuItem::separator())
        .add_item(MenuItem::back())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_menu_tree_has_all_menus() {
        let tree = build_menu_tree();

        assert!(tree.contains_key("main"));
        assert!(tree.contains_key("production"));
        assert!(tree.contains_key("milestone"));
        assert!(tree.contains_key("refactor"));
        assert!(tree.contains_key("pending"));
        assert!(tree.contains_key("navigate"));
    }

    #[test]
    fn main_menu_structure() {
        let tree = build_menu_tree();
        let menu = tree.get("main").unwrap();

        assert_eq!(menu.id, "main");
        assert_eq!(menu.title, "Menu");
        assert!(menu.items.len() >= 5); // At least 5 items

        // Has quit with shortcut
        let quit = menu.get_item("quit").unwrap();
        assert_eq!(quit.shortcut, Some('q'));
    }

    #[test]
    fn production_menu_structure() {
        let tree = build_menu_tree();
        let menu = tree.get("production").unwrap();

        assert!(menu.get_item("start").is_some());
        assert!(menu.get_item("pause").is_some());
        assert!(menu.get_item("milestone_ctrl").is_some());
        assert!(menu.get_item("clear_limit").is_some());
        assert!(menu.get_item("back").is_some());
    }

    #[test]
    fn milestone_menu_has_all_milestones() {
        let tree = build_menu_tree();
        let menu = tree.get("milestone").unwrap();

        for i in 1..=9 {
            let item = menu.get_item(&format!("m{}", i));
            assert!(item.is_some(), "Missing milestone M{}", i);
        }

        // Has back
        assert!(menu.get_item("back").is_some());
    }

    #[test]
    fn refactor_menu_structure() {
        let tree = build_menu_tree();
        let menu = tree.get("refactor").unwrap();

        assert!(menu.get_item("exit_refactor").is_some());
        assert!(menu.get_item("cancel_refactor").is_some());
        assert!(menu.get_item("back").is_some());
    }

    #[test]
    fn pending_menu_structure() {
        let tree = build_menu_tree();
        let menu = tree.get("pending").unwrap();

        assert!(menu.get_item("apply").is_some());
        assert!(menu.get_item("discard").is_some());
        assert!(menu.get_item("review").is_some());
        assert!(menu.get_item("back").is_some());
    }

    #[test]
    fn navigate_menu_structure() {
        let tree = build_menu_tree();
        let menu = tree.get("navigate").unwrap();

        assert!(menu.get_item("nav_home").is_some());
        assert!(menu.get_item("nav_feed").is_some());
        assert!(menu.get_item("nav_main").is_some());
        assert!(menu.get_item("nav_logs").is_some());
        assert!(menu.get_item("nav_tasks").is_some());
        assert!(menu.get_item("nav_agents").is_some());
        assert!(menu.get_item("nav_costs").is_some());
        assert!(menu.get_item("back").is_some());
    }

    #[test]
    fn submenus_reference_valid_ids() {
        let tree = build_menu_tree();

        for menu in tree.values() {
            for item in &menu.items {
                if let super::super::types::MenuItemType::Submenu(ref id) = item.item_type {
                    assert!(
                        tree.contains_key(id),
                        "Menu '{}' references invalid submenu '{}'",
                        menu.id,
                        id
                    );
                }
            }
        }
    }

    #[test]
    fn all_menus_have_back_except_main() {
        let tree = build_menu_tree();

        for (id, menu) in &tree {
            if id != "main" {
                assert!(
                    menu.get_item("back").is_some(),
                    "Menu '{}' missing back item",
                    id
                );
            }
        }
    }
}
