use super::*;

#[test]
fn daemon_recovery_marks_unrecoverable_work_lost_and_clears_live_attachments() {
    let mut prior = snapshot();
    prior.jobs.push(crate::daemon::JobSummary {
        id: crate::daemon::JobId(1),
        kind: crate::daemon::JobKind::BitBakeBuild,
        label: "core-image-minimal".into(),
        lifecycle: LifecycleState::Running,
        progress_current: Some(2),
        progress_total: Some(10),
        exit_code: None,
    });
    prior.pty_sessions.push(crate::daemon::PtySessionSummary {
        id: crate::daemon::PtySessionId(7),
        name: "devshell".into(),
        kind: PtyKind::Devshell,
        cwd: "/work/build".into(),
        lifecycle: LifecycleState::Running,
        dimensions: TerminalDimensions {
            columns: 120,
            rows: 40,
        },
        writer: Some(crate::daemon::ClientId([9; 16])),
        writer_epoch: 4,
        viewers: 2,
        exit_code: None,
        restartable: true,
    });
    let persisted = DaemonPersistedState::capture(
        &prior,
        99,
        "old-boot".into(),
        Vec::new(),
        PersistedPreferences::default(),
    );
    let mut current = snapshot();
    current.daemon_instance_id = DaemonInstanceId([8; 16]);
    current.sequence = 0;
    current.generation = 0;
    let (recovered, report) = recover_persisted_snapshot(current, &persisted, "new-boot");

    assert_eq!(recovered.daemon_instance_id, DaemonInstanceId([8; 16]));
    assert_eq!(recovered.jobs[0].lifecycle, LifecycleState::Lost);
    assert_eq!(recovered.pty_sessions[0].lifecycle, LifecycleState::Lost);
    assert_eq!(recovered.pty_sessions[0].writer, None);
    assert_eq!(recovered.pty_sessions[0].viewers, 0);
    assert_eq!(recovered.bitbake.lifecycle, LifecycleState::Disconnected);
    assert!(recovered.bitbake.diagnostic.is_some());
    assert!(recovered.clients.is_empty());
    assert_eq!(report.lost_jobs, 1);
    assert_eq!(report.lost_terminal_sessions, 1);
    assert!(report.previous_boot_changed);
    assert_eq!(report.boundary, DaemonRecoveryBoundary::HostReboot);
    assert!(report.bitbake_reconnect_recommended);
    assert_eq!(report.terminal_relaunch_intents.len(), 1);
    assert_eq!(report.terminal_relaunch_intents[0].name, "devshell");
    assert_eq!(report.terminal_relaunch_intents[0].cwd, "/work/build");
}
