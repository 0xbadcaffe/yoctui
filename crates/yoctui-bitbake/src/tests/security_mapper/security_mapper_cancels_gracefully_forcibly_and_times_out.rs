use super::*;

#[tokio::test]
async fn security_mapper_cancels_gracefully_forcibly_and_times_out() {
    let directory = TestDirectory::new("control");
    let preview = preview(&directory);
    let SecurityOperation::PackageMap { executable, .. } = &preview.operation else {
        unreachable!();
    };
    write_executable(
        executable,
        "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n",
    );
    let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
    let mut runner =
        SecurityMapperJobRunner::new().with_cancellation_timeout(Duration::from_secs(1));
    runner.start(command.clone()).await.unwrap();
    let _ = runner.next_event().await.unwrap();
    let _ = runner.next_event().await.unwrap();
    assert!(!runner.cancel(SecuritySessionId(8)).await.unwrap());
    assert!(matches!(
        runner.next_event().await.unwrap(),
        SecurityMapperRunnerEvent::CancellationRejected {
            id: SecuritySessionId(8),
            ..
        }
    ));
    assert!(runner.cancel(SecuritySessionId(7)).await.unwrap());
    assert!(matches!(
        runner.next_event().await.unwrap(),
        SecurityMapperRunnerEvent::CancellationRequested {
            id: SecuritySessionId(7)
        }
    ));
    assert!(matches!(
        runner.next_event().await.unwrap(),
        SecurityMapperRunnerEvent::Cancelled {
            id: SecuritySessionId(7),
            forced: false,
            ..
        }
    ));

    write_executable(
        executable,
        "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
    );
    let command = SecurityMapperCommandSpec::from_preview(&preview).unwrap();
    let mut forced =
        SecurityMapperJobRunner::new().with_cancellation_timeout(Duration::from_millis(20));
    forced.start(command.clone()).await.unwrap();
    let _ = forced.next_event().await.unwrap();
    let _ = forced.next_event().await.unwrap();
    assert!(forced.cancel(SecuritySessionId(7)).await.unwrap());
    let _ = forced.next_event().await.unwrap();
    assert!(matches!(
        forced.next_event().await.unwrap(),
        SecurityMapperRunnerEvent::Cancelled { forced: true, .. }
    ));

    let mut timed_out = SecurityMapperJobRunner::new()
        .with_cancellation_timeout(Duration::from_millis(20))
        .with_operation_timeout(Duration::from_millis(20));
    timed_out.start(command).await.unwrap();
    let _ = timed_out.next_event().await.unwrap();
    let _ = timed_out.next_event().await.unwrap();
    assert!(matches!(
        timed_out.next_event().await.unwrap(),
        SecurityMapperRunnerEvent::TimedOut {
            id: SecuritySessionId(7),
            forced: true,
            ..
        }
    ));
}
