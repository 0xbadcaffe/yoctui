use super::*;

#[tokio::test]
async fn raw_job_supervisor_journals_single_graceful_cancellation() {
    let fixture = Fixture::new(
        "cancel",
        "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
    );
    let authority = authority(&fixture);
    let wire = request(&authority, "raw-request:daemon-cancel");
    let mut supervisor = DaemonRawSupervisor::default()
        .with_timeouts(Duration::from_secs(2), Duration::from_secs(1));
    supervisor.replace_compatibility(Some(authority)).unwrap();
    supervisor.start(wire.clone()).unwrap();
    loop {
        let event = next_event(&mut supervisor).await;
        if event.state.phase == yoctui_model::RawExecutionPhase::Running
            && !event.state.stdout.chunks.is_empty()
        {
            break;
        }
    }
    supervisor.cancel(&wire.request_id).unwrap();
    assert!(matches!(
        supervisor.cancel(&wire.request_id),
        Err(DaemonRawError::CancellationAlreadyRequested(_))
    ));
    let mut saw_cancelling = false;
    loop {
        let event = next_event(&mut supervisor).await;
        saw_cancelling |= event.state.phase == yoctui_model::RawExecutionPhase::Cancelling;
        if event.state.phase.is_terminal() {
            assert_eq!(
                event.state.phase,
                yoctui_model::RawExecutionPhase::Terminal(RawExecutionOutcome::Cancelled)
            );
            assert!(saw_cancelling);
            break;
        }
    }
}
