use super::*;
use crate::client_runtime::{PendingTerminalCompletion, TerminalCompletionOutcome};

#[test]
fn dtc_decompile_completion_opens_only_successful_output() {
    let completion = yoctui_model::TerminalCompletion::OpenDeviceTree {
        component: yoctui_model::PlatformComponent::Kernel,
        path: "/tmp/board.dts".into(),
    };
    let pending = PendingTerminalCompletion {
        name: "decompile kernel device tree".into(),
        known_sessions: vec![1],
        session_id: Some(2),
        completion: completion.clone(),
    };
    let mut app = App::new(8, 512);
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 2,
            name: pending.name.clone(),
            lifecycle: ClientDaemonLifecycle::Exited,
            viewers: 0,
        });
    app.daemon
        .pty_details
        .push(yoctui_model::ClientDaemonPtyDetails {
            id: 2,
            kind: yoctui_model::ClientDaemonPtyKind::Utility,
            cwd: "/tmp".into(),
            columns: 80,
            rows: 24,
            writer: None,
            writer_epoch: 0,
            exit_code: Some(0),
            restartable: false,
        });
    assert!(matches!(
        super::super::replica::terminal_completion_outcome(&app, &pending, 2),
        Some(TerminalCompletionOutcome::Succeeded(value)) if value == completion
    ));
    app.daemon.pty_details[0].exit_code = Some(1);
    assert!(matches!(
        super::super::replica::terminal_completion_outcome(&app, &pending, 2),
        Some(TerminalCompletionOutcome::Failed(message)) if message.contains("status 1")
    ));
}
