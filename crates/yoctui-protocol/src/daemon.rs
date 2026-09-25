//! Typed, bounded protocol for the persistent daemon and attachable clients.
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    path::{Component, Path},
};
use thiserror::Error;

use crate::{TaskStatsData, WorkspaceData};

pub const PROTOCOL_MAJOR: u16 = 1;
pub const PROTOCOL_MINOR: u16 = 4;
pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_CAPABILITIES: usize = 128;
pub const MAX_RETAINED_EVENTS: usize = 65_536;
pub const MAX_SNAPSHOT_LOGS: usize = 100_000;
pub const MAX_DAEMON_CLIENTS: usize = 32;
pub const MAX_DAEMON_PTY_SESSIONS: usize = 64;
pub const MAX_TERMINAL_SCROLLBACK_LINES: usize = 100_000;
pub const MAX_TERMINAL_ROWS: u16 = 512;
pub const MAX_TERMINAL_COLUMNS: u16 = 512;
pub const MAX_TERMINAL_CELLS: usize = 250_000;
pub const MAX_TERMINAL_CELL_BYTES: usize = 1_024;
pub const MAX_UTILITY_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PTY_OUTPUT_EVENT_BYTES: usize = 64 * 1024;
pub const MAX_PTY_INPUT_BYTES: usize = 64 * 1024;
pub const MAX_DAEMON_BUILD_EVENTS: usize = 2_048;
pub const COMPATIBILITY_SCHEMA_VERSION: u16 = 1;
pub const MAX_COMPATIBILITY_CAPABILITIES: usize = 512;
pub const MAX_COMPATIBILITY_EVIDENCE: usize = 32;
pub const MAX_COMPATIBILITY_ITEMS: usize = 256;
pub const MAX_COMPATIBILITY_TEXT_BYTES: usize = 4_096;
pub const MAX_COMPATIBILITY_ARGV: usize = 64;
pub const RAW_EXECUTION_SCHEMA_VERSION: u16 = 1;
pub const RAW_HISTORY_SCHEMA_VERSION: u16 = 1;
pub const MAX_RAW_HISTORY_RECORDS: usize = 256;
pub const MAX_RAW_HISTORY_AGGREGATE_BYTES: usize = 256 * 1024;
pub const MAX_RAW_EXECUTION_ID_BYTES: usize = 96;
pub const MAX_RAW_EXECUTION_REQUESTS: usize = 64;
pub const MAX_RAW_EXECUTION_PARAMETERS: usize = 32;
pub const MAX_RAW_EXECUTION_PARAMETER_ID_BYTES: usize = 96;
pub const MAX_RAW_EXECUTION_PARAMETER_BYTES: usize = 4_096;
pub const MAX_RAW_EXECUTION_ARGUMENTS: usize = 64;
pub const MAX_RAW_EXECUTION_ARGUMENT_BYTES: usize = 512;
pub const MAX_RAW_EXECUTION_ARGUMENT_AGGREGATE_BYTES: usize = 8_192;
pub const MAX_RAW_EXECUTION_BUILD_DIRECTORY_BYTES: usize = 4_096;
pub const MAX_RAW_EXECUTION_OUTPUT_CHUNK_BYTES: usize = 64 * 1_024;
pub const MAX_RAW_EXECUTION_RETAINED_BYTES: usize = 1_024 * 1_024;
pub const MAX_RAW_EXECUTION_RETAINED_LINES: usize = 10_000;
pub const MAX_RAW_EXECUTION_MESSAGE_BYTES: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self {
        major: PROTOCOL_MAJOR,
        minor: PROTOCOL_MINOR,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub [u8; 16]);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DaemonInstanceId(pub [u8; 16]);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PtySessionId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(pub u64);

include!("daemon/raw_request_history.rs");
include!("daemon/raw_events_snapshot.rs");
include!("daemon/devtool_status.rs");
include!("daemon/compatibility_identity.rs");
include!("daemon/compatibility_validation.rs");

include!("daemon/client_commands.rs");
include!("daemon/qa_testing_messages.rs");
include!("daemon/terminal_messages.rs");
include!("daemon/snapshot_types.rs");
include!("daemon/snapshot_journal.rs");
include!("daemon/snapshot_reducer.rs");
include!("daemon/errors_framing.rs");

#[cfg(test)]
#[path = "tests/daemon_test_snapshot/mod.rs"]
mod daemon_test_snapshot_tests;

#[cfg(test)]
#[path = "tests/daemon/mod.rs"]
mod tests;
