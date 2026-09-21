use super::*;
use yoctui_protocol::daemon::*;
fn snapshot() -> DaemonSnapshot {
    DaemonSnapshot {
        daemon_instance_id: DaemonInstanceId([1; 16]),
        sequence: 1,
        generation: 1,
        workspace: None,
        project_profile: ProjectProfileSummary::Absent,
        bitbake: BitBakeState {
            lifecycle: LifecycleState::Running,
            version: None,
            capabilities: Vec::new(),
            diagnostic: None,
        },
        compatibility: None,
        jobs: vec![JobSummary {
            id: JobId(1),
            kind: JobKind::BitBakeBuild,
            label: "image".into(),
            lifecycle: LifecycleState::Exited,
            progress_current: None,
            progress_total: None,
            exit_code: Some(0),
        }],
        raw_executions: Vec::new(),
        raw_history: Vec::new(),
        pty_sessions: Vec::new(),
        pty_screens: Vec::new(),
        clients: Vec::new(),
        recent_logs: Vec::new(),
        build_progress: None,
        recovery_warnings: Vec::new(),
        build_events: vec![
            Event::Reset {
                targets: vec!["image".into()],
            },
            Event::Started {
                started_unix_ms: Some(100),
            },
            Event::Completed {
                success: true,
                exit_code: Some(0),
                finished_unix_ms: Some(500),
            },
        ],
    }
}
mod archive_capture_excludes_other_builds_and_retains_bounded_text;
mod archive_input_respects_modal_focus_and_preserves_live_state;
mod archive_legacy_summaries_do_not_invent_logs_or_liveness;
