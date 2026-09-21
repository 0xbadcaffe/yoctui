use super::*;

#[test]
fn client_replica_installs_authority_without_replacing_presentation() {
    use yoctui_protocol::daemon::{
        DaemonEvent, JobId, JobKind, JobSummary, LifecycleState, PtyKind, PtySessionId,
        PtySessionSummary, TerminalDimensions,
    };
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([4; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut initial = daemon_protocol_snapshot(&state);
    initial.bitbake.lifecycle = LifecycleState::Running;
    initial.jobs.push(JobSummary {
        id: JobId(8),
        kind: JobKind::BitBakeBuild,
        label: "core-image-minimal".into(),
        lifecycle: LifecycleState::Running,
        progress_current: Some(3),
        progress_total: Some(10),
        exit_code: None,
    });
    initial.pty_sessions.push(PtySessionSummary {
        id: PtySessionId(9),
        name: "devshell".into(),
        kind: PtyKind::Devshell,
        cwd: "/build".into(),
        lifecycle: LifecycleState::Running,
        dimensions: TerminalDimensions {
            columns: 80,
            rows: 24,
        },
        writer: None,
        writer_epoch: 0,
        viewers: 2,
        exit_code: None,
        restartable: true,
    });
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::Recipes;
    app.focus = FocusTarget::Inspector;
    app.theme = yoctui_model::Theme::MatrixGreen;
    app.dialogs
        .push_back(yoctui_model::Dialog::QuitConfirmation);
    let mut client = DaemonClientSnapshot::default();
    client.begin_synchronization();
    client.replace_app(&mut app, initial);
    assert_eq!(
        app.daemon.status,
        yoctui_model::ClientReplicaStatus::Current
    );
    assert_eq!(
        app.daemon.bitbake,
        yoctui_model::ClientDaemonLifecycle::Running
    );
    assert_eq!(app.daemon.jobs[0].label, "core-image-minimal");
    assert_eq!(app.daemon.pty_sessions[0].viewers, 2);
    assert_eq!(app.screen, Screen::Recipes);
    assert_eq!(app.focus, FocusTarget::Inspector);
    assert_eq!(app.theme, yoctui_model::Theme::MatrixGreen);
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::QuitConfirmation)
    ));

    client
        .apply_event_to_app(
            &mut app,
            &yoctui_protocol::daemon::SequencedEvent {
                sequence: 1,
                generation: 1,
                event: DaemonEvent::JobRemoved { job_id: JobId(8) },
            },
        )
        .unwrap();
    assert!(app.daemon.jobs.is_empty());
    client.disconnect_app(&mut app);
    assert_eq!(
        app.daemon.status,
        yoctui_model::ClientReplicaStatus::Disconnected
    );
    assert_eq!(app.screen, Screen::Recipes);
}
