use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raw_pty_explicit_termination_is_required_and_reports_cancelled() {
    let fixture = Fixture::new(
        "pty-terminate",
        "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
    );
    let authority = authority_for(&fixture, RawInteractionMode::InteractivePty);
    let wire = pty_request(&authority, "raw-request:daemon-pty-terminate");
    let mut raw = DaemonRawSupervisor::default();
    raw.replace_compatibility(Some(authority)).unwrap();
    let start = raw.prepare_pty(wire).unwrap();
    let mut pty = crate::daemon_pty::DaemonPtySupervisor::default();
    pty.start_raw(
        start.pty_id,
        &start.command,
        yoctui_protocol::daemon::TerminalDimensions {
            columns: 80,
            rows: 24,
        },
    )
    .unwrap();
    raw.activate_pty(&start).unwrap();
    loop {
        if let crate::daemon_pty::DaemonPtyEvent::Started { session_id, .. } =
            next_pty_event(&mut pty).await
        {
            raw.pty_started(session_id).unwrap();
            break;
        }
    }

    let DaemonRawCancel::Pty { pty_id, state } =
        raw.cancel(start.state.request.id.as_str()).unwrap()
    else {
        panic!("expected PTY cancellation");
    };
    assert_eq!(state.phase, yoctui_model::RawExecutionPhase::Cancelling);
    pty.terminate(pty_id).unwrap();
    let exit_code = loop {
        match next_pty_event(&mut pty).await {
            crate::daemon_pty::DaemonPtyEvent::Exited { exit_code, .. } => break exit_code,
            crate::daemon_pty::DaemonPtyEvent::Lost { message, .. } => panic!("{message}"),
            _ => {}
        }
    };
    let terminal = raw.pty_finished(pty_id, exit_code, None).unwrap().unwrap();
    assert_eq!(
        terminal.phase,
        yoctui_model::RawExecutionPhase::Terminal(RawExecutionOutcome::Cancelled)
    );
}
