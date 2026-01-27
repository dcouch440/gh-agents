//! TUI application modes.
//!
//! The application can be in different modes that affect:
//! - What commands are available
//! - How the status bar is displayed
//! - What happens when the user types

use crate::types::RefactorSession;

/// Application operating mode
#[derive(Debug, Clone, Default)]
pub enum AppMode {
    /// Normal operation - orchestrating work
    #[default]
    Normal,
    /// Refactor mode - modifying the project plan
    Refactor(RefactorModeState),
}

impl AppMode {
    /// Check if in normal mode
    pub fn is_normal(&self) -> bool {
        matches!(self, AppMode::Normal)
    }

    /// Check if in refactor mode
    pub fn is_refactor(&self) -> bool {
        matches!(self, AppMode::Refactor(_))
    }

    /// Get the mode name for display
    pub fn name(&self) -> &'static str {
        match self {
            AppMode::Normal => "Normal",
            AppMode::Refactor(_) => "Refactor",
        }
    }

    /// Get a short status indicator
    pub fn status_indicator(&self) -> &'static str {
        match self {
            AppMode::Normal => "",
            AppMode::Refactor(state) if state.production_halted => "[REFACTOR - HALTED]",
            AppMode::Refactor(_) => "[REFACTOR]",
        }
    }
}

/// State for refactor mode
#[derive(Debug, Clone, Default)]
pub struct RefactorModeState {
    /// The active refactor session
    pub session: Option<RefactorSession>,
    /// Whether production has been halted
    pub production_halted: bool,
    /// Number of pending changes
    pub pending_changes: usize,
    /// Number of approved changes
    pub approved_changes: usize,
}

impl RefactorModeState {
    /// Create state from a session
    pub fn from_session(session: RefactorSession) -> Self {
        let pending = session.pending_changes().len();
        let approved = session.approved_changes().len();
        let halted = session.production_halted;

        Self {
            session: Some(session),
            production_halted: halted,
            pending_changes: pending,
            approved_changes: approved,
        }
    }

    /// Update state from a session
    pub fn update_from_session(&mut self, session: &RefactorSession) {
        self.production_halted = session.production_halted;
        self.pending_changes = session.pending_changes().len();
        self.approved_changes = session.approved_changes().len();
    }

    /// Get a summary string for display
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if self.production_halted {
            parts.push("Production halted".to_string());
        }

        if self.pending_changes > 0 {
            parts.push(format!("{} pending", self.pending_changes));
        }

        if self.approved_changes > 0 {
            parts.push(format!("{} approved", self.approved_changes));
        }

        if parts.is_empty() {
            "Ready".to_string()
        } else {
            parts.join(" | ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_is_normal() {
        let mode = AppMode::default();
        assert!(mode.is_normal());
        assert!(!mode.is_refactor());
    }

    #[test]
    fn mode_names() {
        assert_eq!(AppMode::Normal.name(), "Normal");
        assert_eq!(AppMode::Refactor(RefactorModeState::default()).name(), "Refactor");
    }

    #[test]
    fn refactor_mode_status_indicator() {
        assert_eq!(AppMode::Normal.status_indicator(), "");

        let state = RefactorModeState::default();
        assert_eq!(AppMode::Refactor(state).status_indicator(), "[REFACTOR]");

        let mut halted_state = RefactorModeState::default();
        halted_state.production_halted = true;
        assert_eq!(
            AppMode::Refactor(halted_state).status_indicator(),
            "[REFACTOR - HALTED]"
        );
    }

    #[test]
    fn refactor_state_summary() {
        let state = RefactorModeState::default();
        assert_eq!(state.summary(), "Ready");

        let mut state = RefactorModeState::default();
        state.production_halted = true;
        state.pending_changes = 2;
        assert_eq!(state.summary(), "Production halted | 2 pending");

        let mut state = RefactorModeState::default();
        state.approved_changes = 1;
        assert_eq!(state.summary(), "1 approved");
    }
}
