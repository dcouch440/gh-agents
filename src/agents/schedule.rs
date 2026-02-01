//! Scheduled agents and event-driven triggers.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::agent::AgentId;

/// Unique identifier for a schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScheduleId(pub Uuid);

impl Default for ScheduleId {
    fn default() -> Self {
        Self::new()
    }
}

impl ScheduleId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Unique identifier for a trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TriggerId(pub Uuid);

impl Default for TriggerId {
    fn default() -> Self {
        Self::new()
    }
}

impl TriggerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// A periodic schedule that assigns a task to an agent at a fixed interval.
#[derive(Debug, Clone)]
pub struct Schedule {
    pub id: ScheduleId,
    pub name: String,
    pub agent_id: AgentId,
    pub interval_seconds: u64,
    pub task_title: String,
    pub task_description: String,
    pub role: Option<String>,
    pub enabled: bool,
    pub last_run_at: Option<DateTime<Utc>>,
}

/// Event types that can fire a trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerEvent {
    TaskCompleted,
    TaskFailed,
}

impl TriggerEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriggerEvent::TaskCompleted => "task_completed",
            TriggerEvent::TaskFailed => "task_failed",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "task_completed" => Some(TriggerEvent::TaskCompleted),
            "task_failed" => Some(TriggerEvent::TaskFailed),
            _ => None,
        }
    }
}

/// An event-driven trigger that assigns a task to an agent when an event occurs.
#[derive(Debug, Clone)]
pub struct Trigger {
    pub id: TriggerId,
    pub name: String,
    pub event_type: TriggerEvent,
    pub agent_id: AgentId,
    pub task_title: String,
    pub task_description: String,
    pub role: Option<String>,
}

/// Manages schedules and triggers.
#[derive(Debug, Default)]
pub struct ScheduleManager {
    schedules: HashMap<ScheduleId, Schedule>,
    triggers: HashMap<TriggerId, Trigger>,
}

impl ScheduleManager {
    pub fn new() -> Self {
        Self::default()
    }

    // --- Schedules ---

    /// Create a new schedule, returning its ID.
    pub fn create_schedule(&mut self, name: String, agent_id: AgentId, interval_seconds: u64, task_title: String, task_description: String, role: Option<String>) -> ScheduleId {
        let id = ScheduleId::new();
        self.schedules.insert(
            id,
            Schedule {
                id,
                name,
                agent_id,
                interval_seconds,
                task_title,
                task_description,
                role,
                enabled: true,
                last_run_at: None,
            },
        );
        id
    }

    /// Create a schedule with a specific ID (for DB reconstruction).
    #[allow(clippy::too_many_arguments)]
    pub fn create_schedule_with_id(
        &mut self,
        id: ScheduleId,
        name: String,
        agent_id: AgentId,
        interval_seconds: u64,
        task_title: String,
        task_description: String,
        role: Option<String>,
        enabled: bool,
        last_run_at: Option<DateTime<Utc>>,
    ) {
        self.schedules.insert(
            id,
            Schedule {
                id,
                name,
                agent_id,
                interval_seconds,
                task_title,
                task_description,
                role,
                enabled,
                last_run_at,
            },
        );
    }

    /// Delete a schedule.
    pub fn delete_schedule(&mut self, id: ScheduleId) -> Result<(), ScheduleError> {
        self.schedules.remove(&id).map(|_| ()).ok_or(ScheduleError::ScheduleNotFound(id))
    }

    /// Enable or disable a schedule.
    pub fn set_enabled(&mut self, id: ScheduleId, enabled: bool) -> Result<(), ScheduleError> {
        let schedule = self.schedules.get_mut(&id).ok_or(ScheduleError::ScheduleNotFound(id))?;
        schedule.enabled = enabled;
        Ok(())
    }

    /// List all schedules.
    pub fn list_schedules(&self) -> Vec<&Schedule> {
        self.schedules.values().collect()
    }

    /// Get schedules that are due to run (enabled and interval elapsed since last run).
    pub fn get_due_schedules(&self, now: DateTime<Utc>) -> Vec<&Schedule> {
        self.schedules
            .values()
            .filter(|s| {
                if !s.enabled {
                    return false;
                }
                match s.last_run_at {
                    None => true, // never run → due immediately
                    Some(last) => {
                        let elapsed = (now - last).num_seconds();
                        elapsed >= s.interval_seconds as i64
                    }
                }
            })
            .collect()
    }

    /// Mark a schedule as having just run.
    pub fn mark_run(&mut self, id: ScheduleId, now: DateTime<Utc>) {
        if let Some(schedule) = self.schedules.get_mut(&id) {
            schedule.last_run_at = Some(now);
        }
    }

    /// Get a schedule by ID.
    pub fn get_schedule(&self, id: &ScheduleId) -> Option<&Schedule> {
        self.schedules.get(id)
    }

    // --- Triggers ---

    /// Create a new trigger, returning its ID.
    pub fn create_trigger(&mut self, name: String, event_type: TriggerEvent, agent_id: AgentId, task_title: String, task_description: String, role: Option<String>) -> TriggerId {
        let id = TriggerId::new();
        self.triggers.insert(
            id,
            Trigger {
                id,
                name,
                event_type,
                agent_id,
                task_title,
                task_description,
                role,
            },
        );
        id
    }

    /// Create a trigger with a specific ID (for DB reconstruction).
    #[allow(clippy::too_many_arguments)]
    pub fn create_trigger_with_id(
        &mut self,
        id: TriggerId,
        name: String,
        event_type: TriggerEvent,
        agent_id: AgentId,
        task_title: String,
        task_description: String,
        role: Option<String>,
    ) {
        self.triggers.insert(
            id,
            Trigger {
                id,
                name,
                event_type,
                agent_id,
                task_title,
                task_description,
                role,
            },
        );
    }

    /// Delete a trigger.
    pub fn delete_trigger(&mut self, id: TriggerId) -> Result<(), ScheduleError> {
        self.triggers.remove(&id).map(|_| ()).ok_or(ScheduleError::TriggerNotFound(id))
    }

    /// List all triggers.
    pub fn list_triggers(&self) -> Vec<&Trigger> {
        self.triggers.values().collect()
    }

    /// Get triggers that match a specific event type.
    pub fn get_triggers_for_event(&self, event_type: TriggerEvent) -> Vec<&Trigger> {
        self.triggers.values().filter(|t| t.event_type == event_type).collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    #[error("Schedule not found: {0:?}")]
    ScheduleNotFound(ScheduleId),
    #[error("Trigger not found: {0:?}")]
    TriggerNotFound(TriggerId),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(n: u128) -> AgentId {
        AgentId(Uuid::from_u128(n))
    }

    #[test]
    fn create_and_list_schedules() {
        let mut mgr = ScheduleManager::new();
        let id = mgr.create_schedule("hourly-tests".into(), agent(1), 3600, "Run tests".into(), "Run the full test suite".into(), None);
        let schedules = mgr.list_schedules();
        assert_eq!(schedules.len(), 1);
        assert_eq!(schedules[0].name, "hourly-tests");
        assert_eq!(schedules[0].id, id);
        assert!(schedules[0].enabled);
    }

    #[test]
    fn toggle_schedule() {
        let mut mgr = ScheduleManager::new();
        let id = mgr.create_schedule("s".into(), agent(1), 60, "t".into(), "d".into(), None);
        assert!(mgr.get_schedule(&id).unwrap().enabled);

        mgr.set_enabled(id, false).unwrap();
        assert!(!mgr.get_schedule(&id).unwrap().enabled);

        mgr.set_enabled(id, true).unwrap();
        assert!(mgr.get_schedule(&id).unwrap().enabled);
    }

    #[test]
    fn delete_schedule() {
        let mut mgr = ScheduleManager::new();
        let id = mgr.create_schedule("s".into(), agent(1), 60, "t".into(), "d".into(), None);
        mgr.delete_schedule(id).unwrap();
        assert!(mgr.list_schedules().is_empty());
    }

    #[test]
    fn delete_nonexistent_schedule_fails() {
        let mut mgr = ScheduleManager::new();
        assert!(mgr.delete_schedule(ScheduleId::new()).is_err());
    }

    #[test]
    fn get_due_schedules_never_run() {
        let mut mgr = ScheduleManager::new();
        mgr.create_schedule("s".into(), agent(1), 3600, "t".into(), "d".into(), None);
        let due = mgr.get_due_schedules(Utc::now());
        assert_eq!(due.len(), 1); // never run → due immediately
    }

    #[test]
    fn get_due_schedules_recently_run() {
        let mut mgr = ScheduleManager::new();
        let id = mgr.create_schedule("s".into(), agent(1), 3600, "t".into(), "d".into(), None);
        let now = Utc::now();
        mgr.mark_run(id, now);

        let due = mgr.get_due_schedules(now);
        assert!(due.is_empty()); // just ran → not due
    }

    #[test]
    fn get_due_schedules_interval_elapsed() {
        let mut mgr = ScheduleManager::new();
        let id = mgr.create_schedule("s".into(), agent(1), 60, "t".into(), "d".into(), None);
        let past = Utc::now() - chrono::Duration::seconds(120);
        mgr.mark_run(id, past);

        let due = mgr.get_due_schedules(Utc::now());
        assert_eq!(due.len(), 1); // 120s > 60s interval → due
    }

    #[test]
    fn disabled_schedule_not_due() {
        let mut mgr = ScheduleManager::new();
        let id = mgr.create_schedule("s".into(), agent(1), 60, "t".into(), "d".into(), None);
        mgr.set_enabled(id, false).unwrap();

        let due = mgr.get_due_schedules(Utc::now());
        assert!(due.is_empty());
    }

    #[test]
    fn create_and_list_triggers() {
        let mut mgr = ScheduleManager::new();
        let id = mgr.create_trigger(
            "on-complete".into(),
            TriggerEvent::TaskCompleted,
            agent(1),
            "Review".into(),
            "Review completed work".into(),
            Some("reviewer".into()),
        );
        let triggers = mgr.list_triggers();
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].id, id);
        assert_eq!(triggers[0].event_type, TriggerEvent::TaskCompleted);
    }

    #[test]
    fn get_triggers_for_event() {
        let mut mgr = ScheduleManager::new();
        mgr.create_trigger("t1".into(), TriggerEvent::TaskCompleted, agent(1), "t".into(), "d".into(), None);
        mgr.create_trigger("t2".into(), TriggerEvent::TaskFailed, agent(2), "t".into(), "d".into(), None);
        mgr.create_trigger("t3".into(), TriggerEvent::TaskCompleted, agent(3), "t".into(), "d".into(), None);

        let completed = mgr.get_triggers_for_event(TriggerEvent::TaskCompleted);
        assert_eq!(completed.len(), 2);

        let failed = mgr.get_triggers_for_event(TriggerEvent::TaskFailed);
        assert_eq!(failed.len(), 1);
    }

    #[test]
    fn delete_trigger() {
        let mut mgr = ScheduleManager::new();
        let id = mgr.create_trigger("t".into(), TriggerEvent::TaskCompleted, agent(1), "t".into(), "d".into(), None);
        mgr.delete_trigger(id).unwrap();
        assert!(mgr.list_triggers().is_empty());
    }

    #[test]
    fn trigger_event_roundtrip() {
        assert_eq!(TriggerEvent::from_db_str(TriggerEvent::TaskCompleted.as_str()), Some(TriggerEvent::TaskCompleted));
        assert_eq!(TriggerEvent::from_db_str(TriggerEvent::TaskFailed.as_str()), Some(TriggerEvent::TaskFailed));
        assert_eq!(TriggerEvent::from_db_str("bogus"), None);
    }

    #[test]
    fn create_schedule_with_id() {
        let mut mgr = ScheduleManager::new();
        let id = ScheduleId::new();
        mgr.create_schedule_with_id(id, "restored".into(), agent(1), 3600, "t".into(), "d".into(), None, true, None);
        assert_eq!(mgr.get_schedule(&id).unwrap().name, "restored");
    }
}
