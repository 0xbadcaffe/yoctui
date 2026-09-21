use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raw_pty_uses_authorized_native_argv_and_survives_detach_resize_and_input() {
    let fixture = Fixture::new(
        "pty",
        "#!/bin/sh\nprintf 'cwd=%s argv=%s\\n' \"$PWD\" \"$*\"\nIFS= read -r line\nstty size\nprintf 'input=%s\\n' \"$line\"\n",
    );
    let authority = authority_for(&fixture, RawInteractionMode::InteractivePty);
    let wire = pty_request(&authority, "raw-request:daemon-pty");
    let mut raw = DaemonRawSupervisor::default();
    raw.replace_compatibility(Some(authority)).unwrap();
    let start = raw.prepare_pty(wire.clone()).unwrap();
    assert_eq!(start.pty_id.0 >> 60, RAW_PTY_NAMESPACE >> 60);
    assert_eq!(start.command.executable(), fixture.executable());
    assert_eq!(
        start.command.arguments(),
        ["-u", "knotty", "core-image-minimal"]
    );
    assert_eq!(start.command.current_directory(), fixture.0);
    assert!(
        raw.prepare_pty(wire.clone()).is_ok(),
        "an unactivated authorization remains spawn-free and retryable"
    );

    let dimensions = yoctui_protocol::daemon::TerminalDimensions {
        columns: 90,
        rows: 30,
    };
    let client = yoctui_model::PtyClientId([41; 16]);
    let mut pty = crate::daemon_pty::DaemonPtySupervisor::default();
    pty.start_raw(start.pty_id, &start.command, dimensions)
        .unwrap();
    raw.activate_pty(&start).unwrap();
    assert!(matches!(
        raw.prepare_pty(wire),
        Err(DaemonRawError::DuplicateRequest(_))
    ));
    pty.attach(start.pty_id, client).unwrap();

    loop {
        match next_pty_event(&mut pty).await {
            crate::daemon_pty::DaemonPtyEvent::Started { session_id, .. } => {
                assert_eq!(session_id, start.pty_id);
                let state = raw.pty_started(session_id).unwrap().unwrap();
                assert_eq!(state.phase, yoctui_model::RawExecutionPhase::Running);
                break;
            }
            crate::daemon_pty::DaemonPtyEvent::Lost { message, .. } => panic!("{message}"),
            _ => {}
        }
    }
    let epoch = pty.take(start.pty_id, client, 0).unwrap();
    pty.resize(
        start.pty_id,
        client,
        epoch,
        yoctui_model::PtyDimensions {
            columns: 100,
            rows: 35,
        },
    )
    .unwrap();
    pty.detach(start.pty_id, client).unwrap();
    let detached = raw.pty_attachment(start.pty_id, false).unwrap().unwrap();
    assert_eq!(detached.attachment, RawAttachmentState::Detached);
    assert_eq!(detached.phase, yoctui_model::RawExecutionPhase::Running);
    pty.attach(start.pty_id, client).unwrap();
    let attached = raw.pty_attachment(start.pty_id, true).unwrap().unwrap();
    assert_eq!(attached.attachment, RawAttachmentState::Attached);
    let epoch = pty
        .take(start.pty_id, client, epoch.saturating_add(1))
        .unwrap();
    pty.input(start.pty_id, client, epoch, b"hello raw pty\n".to_vec())
        .unwrap();

    let mut output = Vec::new();
    let exit_code = loop {
        match next_pty_event(&mut pty).await {
            crate::daemon_pty::DaemonPtyEvent::Output { bytes, .. } => output.extend(bytes),
            crate::daemon_pty::DaemonPtyEvent::Exited { exit_code, .. } => break exit_code,
            crate::daemon_pty::DaemonPtyEvent::Lost { message, .. } => panic!("{message}"),
            _ => {}
        }
    };
    let text = String::from_utf8_lossy(&output);
    assert!(text.contains(&format!("cwd={}", fixture.0.display())));
    assert!(text.contains("argv=-u knotty core-image-minimal"));
    assert!(text.contains("35 100"));
    assert!(text.contains("input=hello raw pty"));
    let terminal = raw
        .pty_finished(start.pty_id, exit_code, None)
        .unwrap()
        .unwrap();
    assert_eq!(
        terminal.phase,
        yoctui_model::RawExecutionPhase::Terminal(RawExecutionOutcome::Succeeded)
    );
    assert!(terminal.stdout.chunks.is_empty());
    assert!(terminal.stderr.chunks.is_empty());
}
