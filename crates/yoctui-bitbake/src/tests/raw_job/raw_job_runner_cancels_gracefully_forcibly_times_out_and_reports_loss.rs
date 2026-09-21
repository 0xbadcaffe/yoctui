use super::*;

#[tokio::test]
async fn raw_job_runner_cancels_gracefully_forcibly_times_out_and_reports_loss() {
    let fixture = Fixture::new("terminal");
    let executable = fixture
        .executable("#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n");
    let authority = fixture_authority(&fixture.0, &executable);
    let mut graceful = RawJobRunner::new().with_cancellation_timeout(Duration::from_secs(1));
    graceful.start(spec(&authority)).await.unwrap();
    graceful.next_event().await.unwrap();
    assert!(matches!(
        graceful.next_event().await.unwrap(),
        RawJobRunnerEvent::Output(_)
    ));
    assert!(graceful.cancel().await.unwrap());
    assert!(!graceful.cancel().await.unwrap());
    assert!(matches!(
        graceful.next_event().await.unwrap(),
        RawJobRunnerEvent::Cancelled { forced: false, .. }
    ));

    let executable =
        fixture.executable("#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n");
    let authority = fixture_authority(&fixture.0, &executable);
    let mut forced = RawJobRunner::new().with_cancellation_timeout(Duration::from_millis(20));
    forced.start(spec(&authority)).await.unwrap();
    forced.next_event().await.unwrap();
    assert!(matches!(
        forced.next_event().await.unwrap(),
        RawJobRunnerEvent::Output(_)
    ));
    assert!(forced.cancel().await.unwrap());
    assert!(matches!(
        forced.next_event().await.unwrap(),
        RawJobRunnerEvent::Cancelled { forced: true, .. }
    ));

    let executable = fixture.executable("#!/bin/sh\nwhile :; do sleep 1; done\n");
    let authority = fixture_authority(&fixture.0, &executable);
    let mut timed = RawJobRunner::new()
        .with_operation_timeout(Duration::from_millis(20))
        .with_cancellation_timeout(Duration::from_millis(20));
    timed.start(spec(&authority)).await.unwrap();
    timed.next_event().await.unwrap();
    assert!(matches!(
        timed.next_event().await.unwrap(),
        RawJobRunnerEvent::TimedOut { .. }
    ));

    let executable = fixture.executable("#!/bin/sh\nwhile :; do sleep 1; done\n");
    let authority = fixture_authority(&fixture.0, &executable);
    let mut lost = RawJobRunner::new();
    lost.start(spec(&authority)).await.unwrap();
    lost.next_event().await.unwrap();
    lost.lose_output_channel();
    assert!(matches!(
        lost.next_event().await.unwrap(),
        RawJobRunnerEvent::Lost { .. }
    ));
}
