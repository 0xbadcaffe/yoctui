use super::*;

#[tokio::test]
async fn test_results_revalidate_tampering_stream_output_and_nonzero() {
    let (_directory, adapter, tool, results) = fixture("runner");
    let baseline_path = results.join("baseline").join("testresults.json");
    let candidate_path = results.join("candidate").join("testresults.json");
    for path in [&baseline_path, &candidate_path] {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
    }
    fs::write(&baseline_path, result_json("PASSED")).unwrap();
    fs::write(&candidate_path, result_json("FAILED")).unwrap();
    let response = imported(
        &adapter,
        1,
        vec![baseline_path.clone(), candidate_path.clone()],
    );
    let baseline = &response.records[0];
    let candidate = &response.records[1];
    let request =
        TestComparisonRequest::new(1, baseline.identity.clone(), candidate.identity.clone())
            .unwrap();
    let preview = TestComparisonPreview::new(tool.clone(), request.clone()).unwrap();
    let stale = adapter
        .comparison_command(&preview, baseline, candidate)
        .unwrap();
    fs::write(&candidate.identity.path, result_json("ERROR")).unwrap();
    assert!(matches!(
        TestResultJob::new().start(stale).await,
        Err(TestResultAdapterError::StaleResult(_))
    ));

    let response = imported(
        &adapter,
        2,
        vec![baseline_path.clone(), candidate_path.clone()],
    );
    let baseline = &response.records[0];
    let candidate = &response.records[1];
    let request =
        TestComparisonRequest::new(2, baseline.identity.clone(), candidate.identity.clone())
            .unwrap();
    executable(
        &tool,
        "#!/bin/sh\nprintf 'stdout\\n'\nprintf 'stderr\\n' >&2\nexit 7\n",
    );
    let preview = TestComparisonPreview::new(tool, request.clone()).unwrap();
    let command = adapter
        .comparison_command(&preview, baseline, candidate)
        .unwrap();
    let mut runner = TestResultJob::new();
    runner.start(command.clone()).await.unwrap();
    assert_eq!(
        runner.start(command).await,
        Err(TestResultAdapterError::Busy)
    );
    assert_eq!(
        runner.next_event().await.unwrap(),
        TestResultRunnerEvent::Started {
            operation: TestResultOperation::Comparison(request.clone())
        }
    );
    let mut stdout = false;
    let mut stderr = false;
    loop {
        match runner.next_event().await.unwrap() {
            TestResultRunnerEvent::Output { stream, .. } => {
                stdout |= stream == TestOutputStream::Stdout;
                stderr |= stream == TestOutputStream::Stderr;
            }
            TestResultRunnerEvent::Failed {
                operation,
                exit_code,
            } => {
                assert_eq!(operation, TestResultOperation::Comparison(request));
                assert_eq!(exit_code, Some(7));
                break;
            }
            event => panic!("unexpected resulttool event: {event:?}"),
        }
    }
    assert!(stdout && stderr);
}
