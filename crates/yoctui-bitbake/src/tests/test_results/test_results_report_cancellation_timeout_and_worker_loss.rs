use super::*;

#[tokio::test]
async fn test_results_report_cancellation_timeout_and_worker_loss() {
    let (_directory, adapter, tool, results) = fixture("control");
    let baseline_path = results.join("baseline").join("testresults.json");
    let candidate_path = results.join("candidate").join("testresults.json");
    for path in [&baseline_path, &candidate_path] {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
    }
    fs::write(&baseline_path, result_json("PASSED")).unwrap();
    fs::write(&candidate_path, result_json("FAILED")).unwrap();
    let response = imported(&adapter, 1, vec![baseline_path, candidate_path]);
    let baseline = &response.records[0];
    let candidate = &response.records[1];
    let request =
        TestComparisonRequest::new(1, baseline.identity.clone(), candidate.identity.clone())
            .unwrap();

    executable(
        &tool,
        "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n",
    );
    let preview = TestComparisonPreview::new(tool.clone(), request.clone()).unwrap();
    let command = adapter
        .comparison_command(&preview, baseline, candidate)
        .unwrap();
    let mut cancelled = TestResultJob::new().with_cancellation_timeout(Duration::from_secs(1));
    cancelled.start(command).await.unwrap();
    let _ = cancelled.next_event().await.unwrap();
    let _ = cancelled.next_event().await.unwrap();
    assert!(cancelled.cancel().await.unwrap());
    assert!(matches!(
        cancelled.next_event().await.unwrap(),
        TestResultRunnerEvent::Cancelled { forced: false, .. }
    ));

    executable(
        &tool,
        "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
    );
    let preview = TestComparisonPreview::new(tool.clone(), request.clone()).unwrap();
    let command = adapter
        .comparison_command(&preview, baseline, candidate)
        .unwrap();
    let mut timed_out = TestResultJob::new()
        .with_cancellation_timeout(Duration::from_millis(20))
        .with_operation_timeout(Duration::from_millis(20));
    timed_out.start(command.clone()).await.unwrap();
    let _ = timed_out.next_event().await.unwrap();
    loop {
        if matches!(
            timed_out.next_event().await.unwrap(),
            TestResultRunnerEvent::TimedOut { forced: true, .. }
        ) {
            break;
        }
    }

    let mut lost = TestResultJob::new();
    lost.start(command).await.unwrap();
    let _ = lost.next_event().await.unwrap();
    lost.lose_output_channel();
    assert!(matches!(
        lost.next_event().await.unwrap(),
        TestResultRunnerEvent::Lost { .. }
    ));
    assert!(!lost.cancel().await.unwrap());
    assert!(matches!(
        lost.next_event().await.unwrap(),
        TestResultRunnerEvent::CancellationRejected { .. }
    ));
}
