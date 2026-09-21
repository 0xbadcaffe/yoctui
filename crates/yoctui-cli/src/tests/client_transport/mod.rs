use std::{fs, os::unix::fs::PermissionsExt, thread, time::Duration};

use super::*;
use yoctui_protocol::{
    daemon::{
        BitBakeState, Capability, ClientId, ClientMessage, CommandOutcome, CommandRequest,
        CommandResult, DaemonCommand, DaemonHello, DaemonInstanceId, DaemonSnapshot,
        LifecycleState, MAX_FRAME_BYTES, ProjectProfileSummary, ProtocolLimits, ProtocolVersion,
        RequestId, ResumeCursor, ServerMessage, Subscription,
    },
    daemon_ipc::{DaemonListener, runtime_paths_for},
};

fn snapshot(instance: DaemonInstanceId) -> DaemonSnapshot {
    DaemonSnapshot {
        daemon_instance_id: instance,
        sequence: 0,
        generation: 0,
        workspace: None,
        project_profile: ProjectProfileSummary::NotLoaded,
        bitbake: BitBakeState {
            lifecycle: LifecycleState::Disconnected,
            version: None,
            capabilities: Vec::new(),
            diagnostic: None,
        },
        compatibility: None,
        jobs: Vec::new(),
        raw_executions: Vec::new(),
        raw_history: Vec::new(),
        pty_sessions: Vec::new(),
        pty_screens: Vec::new(),
        clients: Vec::new(),
        recent_logs: Vec::new(),
        build_events: Vec::new(),
        build_progress: None,
        recovery_warnings: Vec::new(),
    }
}

fn hello(instance: DaemonInstanceId) -> DaemonHello {
    DaemonHello {
        selected_version: ProtocolVersion::CURRENT,
        daemon_instance_id: instance,
        boot_id: "boot-test".into(),
        capabilities: vec![
            Capability::StateSnapshots,
            Capability::IncrementalEvents,
            Capability::GracefulShutdown,
        ],
        limits: ProtocolLimits {
            maximum_frame_bytes: MAX_FRAME_BYTES as u32,
            maximum_snapshot_bytes: MAX_FRAME_BYTES as u32,
            maximum_pending_requests: 8,
            maximum_queue_depth: 16,
            maximum_terminal_rows: 512,
            maximum_terminal_columns: 512,
            maximum_clients: 32,
            maximum_pty_sessions: 64,
            maximum_scrollback_lines: 100_000,
            maximum_utility_output_bytes: 4 * 1024 * 1024,
        },
    }
}

mod client_transport_negotiates_attaches_correlates_and_detaches;
mod client_transport_rejects_incompatible_or_unbounded_hello;
mod initial_daemon_attach_missing_socket_fails_without_waiting_for_budget;
mod initial_daemon_attach_retains_authority_after_delayed_snapshot;
