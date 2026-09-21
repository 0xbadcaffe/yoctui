use super::*;
use crate::{Action, App, Effect, LogEntry, LogWorkspaceView, Severity, update};
use std::time::Duration;

fn record(index: usize, level: InternalLogLevel, target: &str) -> InternalLogRecord {
    InternalLogRecord {
        id: 0,
        timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(index as u64),
        level,
        target: target.into(),
        message: format!("diagnostic-{index:04}-{}", "界".repeat(40)),
    }
}

mod ux_internal_log_state_is_bounded_filterable_and_viewport_only;

mod ux_internal_log_reducer_keeps_bitbake_authority_separate_and_export_bounded;
