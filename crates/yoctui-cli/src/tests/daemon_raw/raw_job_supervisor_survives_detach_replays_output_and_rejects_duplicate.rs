use super::*;

#[tokio::test]
async fn raw_job_supervisor_survives_detach_replays_output_and_rejects_duplicate() {
    let fixture = Fixture::new(
        "complete",
        "#!/bin/sh\nprintf 'detached output 界\\n'\nexit 0\n",
    );
    let authority = authority(&fixture);
    let wire = request(&authority, "raw-request:daemon-complete");
    let mut supervisor = DaemonRawSupervisor::default();
    supervisor.replace_compatibility(Some(authority)).unwrap();
    let started = supervisor.start(wire.clone()).unwrap();
    assert_eq!(started.state.phase, yoctui_model::RawExecutionPhase::Queued);
    assert!(matches!(
        supervisor.start(wire),
        Err(DaemonRawError::DuplicateRequest(_))
    ));

    // No connection owns or polls the worker during this interval.
    tokio::time::sleep(Duration::from_millis(20)).await;
    let terminal = loop {
        let event = next_event(&mut supervisor).await;
        if event.state.phase.is_terminal() {
            break event.state;
        }
    };
    assert_eq!(
        terminal.phase,
        yoctui_model::RawExecutionPhase::Terminal(RawExecutionOutcome::Succeeded)
    );
    assert!(
        terminal
            .stdout
            .chunks
            .iter()
            .any(|chunk| chunk.text == "detached output 界")
    );
    assert!(matches!(
        supervisor.cancel(terminal.request.id.as_str()),
        Err(DaemonRawError::UnknownRequest(_))
    ));
}
