use super::*;

#[test]
fn notification_transient_status_uses_typed_priority_and_dismisses() {
    let mut app = App::new(10, 1_000);
    assert_eq!(app.transient_status(), None);

    let _ = update(&mut app, Action::Notify("  Saved profile  ".into()));
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Notification,
            text: "Saved profile".into(),
        })
    );

    app.dialogs.push_front(Dialog::QuitConfirmation);
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Confirmation,
            text: "Confirmation pending".into(),
        })
    );
    app.build.status = BuildStatus::Failed;
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Confirmation,
            text: "Confirmation pending".into(),
        })
    );
    app.dialogs.clear();
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Notification)
    );
    let _ = update(
        &mut app,
        Action::Failure(AppError::new("backend", "connection lost", "retry")),
    );
    app.dialogs.push_front(Dialog::QuitConfirmation);
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Error)
    );
    let _ = update(&mut app, Action::DismissNotification);
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Confirmation)
    );
    app.dialogs.clear();
    assert_eq!(app.transient_status(), None);

    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Confirmation)
    );
    app.dialogs.clear();
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Success)
    );
    let _ = update(&mut app, Action::BuildCancelled { exit_code: None });
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Confirmation)
    );
    app.dialogs.clear();
    assert_eq!(
        app.transient_status().map(|status| status.kind),
        Some(TransientStatusKind::Warning)
    );
    app.notification = Some("   ".into());
    app.build.status = BuildStatus::Idle;
    assert_eq!(app.transient_status(), None);

    app.notification = None;
    app.daemon.status = ClientReplicaStatus::Synchronizing;
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Reconnecting,
            text: "Daemon synchronizing".into(),
        })
    );
    app.daemon.status = ClientReplicaStatus::Stale;
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Warning,
            text: "Daemon state stale".into(),
        })
    );
    app.daemon.status = ClientReplicaStatus::Current;
    app.daemon.bitbake = ClientDaemonLifecycle::Connecting;
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Activity,
            text: "BitBake connecting".into(),
        })
    );

    app.daemon.bitbake = ClientDaemonLifecycle::Running;
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(7, true)),
    );
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Activity,
            text: "0 active jobs · 1 queued".into(),
        })
    );
    app.background_jobs.jobs.clear();
    app.build.status = BuildStatus::Running;
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Activity,
            text: "Build running · 0 active".into(),
        })
    );
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(8, true)),
    );
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Activity,
            text: "Build running · 0 active · 1 queued".into(),
        })
    );
}
