use super::*;

#[test]
fn daemon_state_jobs_reuse_existing_typed_families_without_client_presentation() {
    let mut source = App::new(16, 4096);
    source.screen = Screen::Maintenance;
    source.focus = FocusTarget::Inspector;
    source.notification = Some("client-only".into());
    source.logs.insert(crate::LogEntry {
        id: 0,
        severity: crate::Severity::Info,
        message: "daemon-owned log".into(),
        recipe: None,
        task: None,
        path: None,
        timestamp: std::time::SystemTime::UNIX_EPOCH,
        build: None,
        protected: false,
        diagnostic: None,
    });
    let jobs = DaemonJobState::capture(&source);

    let mut replica = App::new(16, 4096);
    replica.screen = Screen::Layers;
    replica.focus = FocusTarget::Navigator;
    replica.notification = Some("keep-local".into());
    jobs.install_replica(&mut replica);

    assert_eq!(replica.logs, source.logs);
    assert_eq!(replica.background_jobs, source.background_jobs);
    assert_eq!(replica.qemu_sessions, source.qemu_sessions);
    assert_eq!(replica.wic_sessions, source.wic_sessions);
    assert_eq!(replica.sdk_sessions, source.sdk_sessions);
    assert_eq!(replica.test_sessions, source.test_sessions);
    assert_eq!(replica.security, source.security);
    assert_eq!(replica.qa, source.qa);
    assert_eq!(replica.maintenance, source.maintenance);
    assert_eq!(replica.screen, Screen::Layers);
    assert_eq!(replica.focus, FocusTarget::Navigator);
    assert_eq!(replica.notification.as_deref(), Some("keep-local"));
    assert!(jobs.pty_sessions.is_empty());
}
