use super::*;

#[tokio::test]
async fn test_runner_rejects_tampering_streams_bounded_output_and_completes() {
    let (_directory, adapter) = fixture("stream");
    let request = request(&adapter, TestFamily::BitbakeSelftest);
    let tool = request.executable.clone();
    let command = adapter.command(&request).unwrap();
    executable(&tool, "#!/bin/sh\nexit 0\n");
    let mut stale = TestRunnerJob::new();
    assert!(matches!(
        stale.start(command).await,
        Err(TestRunnerAdapterError::StaleExecutable(_))
    ));

    executable(
        &tool,
        &format!(
            "#!/bin/sh\nprintf 'env=%s\\n' \"$BB_SKIP_NETTESTS\"\nprintf 'stderr\\n' >&2\nprintf '{}\\n'\nexit 0\n",
            "x".repeat(MAX_TEST_RUNNER_LINE_BYTES + 8)
        ),
    );
    let refreshed = TestSelftestRequest::new(
        tool,
        request.family,
        request.selector,
        request.parallelism,
        request.verbose,
        request.skip_network,
    )
    .unwrap();
    let command = adapter.command(&refreshed).unwrap();
    let mut runner = TestRunnerJob::new();
    runner.start(command.clone()).await.unwrap();
    assert_eq!(
        runner.start(command).await,
        Err(TestRunnerAdapterError::Busy)
    );
    assert_eq!(runner.next_event().await.unwrap(), TestRunnerEvent::Started);
    let mut stdout = false;
    let mut stderr = false;
    let mut truncated = false;
    loop {
        match runner.next_event().await.unwrap() {
            TestRunnerEvent::Output {
                stream,
                line,
                truncated: line_truncated,
            } => {
                stdout |= stream == TestOutputStream::Stdout;
                stderr |= stream == TestOutputStream::Stderr;
                truncated |= line_truncated;
                if line.starts_with("env=") {
                    assert_eq!(line, "env=yes");
                }
            }
            TestRunnerEvent::Completed {
                exit_code,
                result_paths,
            } => {
                assert_eq!(exit_code, Some(0));
                assert!(result_paths.is_empty());
                break;
            }
            event => panic!("unexpected event: {event:?}"),
        }
    }
    assert!(stdout && stderr && truncated);
    assert!(std::env::var_os("BB_SKIP_NETTESTS").is_none());
}
