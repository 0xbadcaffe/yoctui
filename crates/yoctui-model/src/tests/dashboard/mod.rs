use super::*;
use std::time::Duration;

fn log(severity: Severity, index: u64) -> LogEntry {
    LogEntry {
        id: index,
        severity,
        message: format!("diagnostic-{index}"),
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(index),
        build: None,
        protected: false,
        diagnostic: None,
    }
}

mod ux_dashboard_prioritizes_failures_then_active_work_without_duplicating_state;

mod ux_dashboard_projects_environment_build_and_artifact_next_actions_honestly;

mod ux_command_center_borrows_bounded_contexts_work_favorites_and_terminals;
