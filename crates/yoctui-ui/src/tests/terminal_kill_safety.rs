use super::*;
use yoctui_model::{ClientDaemonLifecycle, ClientDaemonPtySummary, ClientReplicaStatus};

#[test]
fn terminal_kill_safety_existing_inline_warning_names_only_the_reviewed_target() {
    let mut app = App::new(8, 1000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.daemon.pty_sessions.push(ClientDaemonPtySummary {
        id: 41,
        name: "owned-demo".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        viewers: 0,
    });
    update(&mut app, Action::TerminalBeginKill);
    let output = rendered_region_rows(160, 30, |frame, area| {
        terminal_workspace::terminal_sessions_workspace(frame, &app, area);
    })
    .join("\n");
    assert!(
        output.contains("KILL #41:owned-demo process group? Enter confirm"),
        "{output}"
    );
    app.daemon.pty_sessions[0].name = "renamed-after-review".into();
    let output = rendered_region_rows(160, 30, |frame, area| {
        terminal_workspace::terminal_sessions_workspace(frame, &app, area);
    })
    .join("\n");
    assert!(output.contains("KILL #41:owned-demo"), "{output}");
    for (width, height) in [(20, 8), (80, 24), (160, 50)] {
        let _ = rendered_text_at(&app, width, height, literal_now());
    }
    update(&mut app, Action::TerminalCancelMode);
    assert!(app.terminal.kill_target.is_none());
}
