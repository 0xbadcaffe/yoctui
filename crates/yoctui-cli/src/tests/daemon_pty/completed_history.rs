use super::*;
use yoctui_protocol::daemon::MAX_DAEMON_PTY_SESSIONS;

fn start(supervisor: &mut DaemonPtySupervisor, code: &str) -> PtySessionId {
    supervisor
        .start_new(
            "bounded report".into(),
            PtyKind::Utility,
            "/tmp".into(),
            PtyCommand {
                program: "/usr/bin/python3".into(),
                arguments: vec!["-u".into(), "-c".into(), code.into()],
                environment_profile_id: None,
            },
            TerminalDimensions {
                columns: 80,
                rows: 8,
            },
        )
        .unwrap()
}

async fn exited(supervisor: &mut DaemonPtySupervisor, id: PtySessionId) -> Vec<PtySessionId> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut removed = Vec::new();
    loop {
        assert!(Instant::now() < deadline, "PTY {} did not exit", id.0);
        if let Some(event) = supervisor.try_event() {
            match event {
                DaemonPtyEvent::Exited {
                    session_id,
                    exit_code,
                    ..
                } if session_id == id => {
                    assert_eq!(exit_code, Some(0));
                    return removed;
                }
                DaemonPtyEvent::Removed { session_id } => removed.push(session_id),
                DaemonPtyEvent::Lost { message, .. } => panic!("{message}"),
                _ => {}
            }
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
}

fn text(screen: &yoctui_protocol::daemon::PtyScreenSnapshot) -> String {
    screen
        .cells
        .iter()
        .map(|cell| cell.contents.as_str())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn completed_pty_preserves_real_scrollback_but_never_accepts_writer_input() {
    let mut supervisor = DaemonPtySupervisor::default();
    let id = start(
        &mut supervisor,
        "for i in range(100): print(f'ROW_{i:03d}')",
    );
    exited(&mut supervisor, id).await;
    let bottom = supervisor.snapshot(id, 0).unwrap();
    let top = supervisor
        .snapshot(id, bottom.scrollback_lines as usize)
        .unwrap();
    assert!(text(&bottom).contains("ROW_099"));
    assert!(!text(&bottom).contains("ROW_000"));
    assert!(text(&top).contains("ROW_000"));
    assert_ne!(bottom.cells, top.cells);
    assert!(top.scrollback_offset > 0);
    let client = PtyClientId([19; 16]);
    supervisor.attach(id, client).unwrap();
    assert!(supervisor.take(id, client, 0).is_err());
    assert!(
        supervisor
            .input(id, client, 0, b"not executable\n".to_vec())
            .is_err()
    );
    supervisor.close(id).unwrap();
    assert!(supervisor.snapshot(id, 0).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn completed_pty_capacity_reclaims_old_reports_without_evicting_a_live_writer() {
    let mut supervisor = DaemonPtySupervisor::default();
    let live = start(
        &mut supervisor,
        "import sys; print('LIVE_READY'); print('ECHO_' + sys.stdin.readline().strip())",
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        assert!(Instant::now() < deadline);
        if matches!(supervisor.try_event(), Some(DaemonPtyEvent::Started { session_id, .. }) if session_id == live)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    let client = PtyClientId([20; 16]);
    supervisor.attach(live, client).unwrap();
    let epoch = supervisor.take(live, client, 0).unwrap();
    let mut removed = Vec::new();
    let mut latest = live;
    for _ in 0..MAX_DAEMON_PTY_SESSIONS + 8 {
        latest = start(&mut supervisor, "print('DONE')");
        removed.extend(exited(&mut supervisor, latest).await);
        assert!(supervisor.sessions.len() <= MAX_DAEMON_PTY_SESSIONS);
    }
    assert!(!removed.is_empty());
    assert!(!removed.contains(&live));
    assert!(supervisor.snapshot(latest, 0).is_ok());
    supervisor
        .input(live, client, epoch, b"LIVE_STILL_PRESENT\n".to_vec())
        .unwrap();
    exited(&mut supervisor, live).await;
    assert!(text(&supervisor.snapshot(live, 0).unwrap()).contains("ECHO_LIVE_STILL_PRESENT"));
}

#[tokio::test]
async fn completed_pty_capacity_never_discards_active_handles_or_on_invalid_requests() {
    let mut supervisor = DaemonPtySupervisor::default();
    let mut receivers = Vec::new();
    for id in 1..=MAX_DAEMON_PTY_SESSIONS {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        supervisor.sessions.insert(
            PtySessionId(id as u64),
            super::super::SessionHandle {
                control: sender,
                ended_at: None,
            },
        );
        receivers.push(receiver);
    }
    assert!(
        supervisor
            .reserve_capacity()
            .unwrap_err()
            .contains("all sessions are active")
    );
    assert_eq!(supervisor.sessions.len(), MAX_DAEMON_PTY_SESSIONS);
    for handle in supervisor.sessions.values_mut() {
        handle.ended_at = Some(Instant::now());
    }
    assert!(
        supervisor
            .start_new(
                "invalid".into(),
                PtyKind::Utility,
                "relative-cwd".into(),
                PtyCommand {
                    program: "relative-program".into(),
                    arguments: Vec::new(),
                    environment_profile_id: None
                },
                TerminalDimensions {
                    columns: 80,
                    rows: 8
                }
            )
            .is_err()
    );
    assert_eq!(supervisor.sessions.len(), MAX_DAEMON_PTY_SESSIONS);
    supervisor.reserve_capacity().unwrap();
    assert_eq!(supervisor.sessions.len(), MAX_DAEMON_PTY_SESSIONS - 1);
    assert!(matches!(
        supervisor.state_rx.try_recv(),
        Ok(DaemonPtyEvent::Removed { .. })
    ));
}
