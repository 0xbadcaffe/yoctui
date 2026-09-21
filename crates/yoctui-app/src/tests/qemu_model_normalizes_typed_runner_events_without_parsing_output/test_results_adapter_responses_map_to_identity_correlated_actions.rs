use super::*;

#[test]
fn test_results_adapter_responses_map_to_identity_correlated_actions() {
    let baseline = yoctui_model::TestResultIdentity::new(
        "/results/baseline/testresults.json".into(),
        10,
        SystemTime::UNIX_EPOCH,
        "baseline".into(),
    )
    .unwrap();
    let candidate = yoctui_model::TestResultIdentity::new(
        "/results/candidate/testresults.json".into(),
        11,
        SystemTime::UNIX_EPOCH,
        "candidate".into(),
    )
    .unwrap();
    let import_request =
        yoctui_model::TestResultImportRequest::new(3, vec![baseline.path.clone()]).unwrap();
    assert_eq!(
        test_results_import_action(TestResultImportResponse {
            request: import_request.clone(),
            records: Vec::new(),
            limitations: vec!["empty fixture".into()],
        }),
        Action::TestResultsLoaded {
            request: import_request,
            records: Vec::new(),
            limitations: vec!["empty fixture".into()],
        }
    );

    let request =
        yoctui_model::TestComparisonRequest::new(4, baseline.clone(), candidate.clone()).unwrap();
    let comparison = TestComparison {
        baseline,
        candidate,
        transitions: Vec::new(),
    };
    assert_eq!(
        test_result_actions_for_runner_event(
            TestResultRunnerEvent::Completed {
                operation: TestResultOperation::Comparison(request.clone()),
                exit_code: Some(0),
            },
            Some(comparison.clone()),
            vec!["bounded import".into()],
        ),
        [Action::TestComparisonLoaded {
            request: request.clone(),
            comparison,
            limitations: vec!["bounded import".into()],
        }]
    );
    assert_eq!(
        test_result_actions_for_runner_event(
            TestResultRunnerEvent::Failed {
                operation: TestResultOperation::Comparison(request.clone()),
                exit_code: Some(6),
            },
            None,
            Vec::new(),
        ),
        [Action::TestComparisonFailed {
            request,
            message: "resulttool exited unsuccessfully with exit code 6".into(),
        }]
    );
}
