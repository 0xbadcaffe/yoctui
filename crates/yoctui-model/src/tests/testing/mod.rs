use super::*;

fn capability() -> TestCapability {
    TestCapability {
        oe_selftest: TestExecutableCapability::Available("/workspace/oe-selftest".into()),
        bitbake_selftest: TestExecutableCapability::Available("/workspace/bitbake-selftest".into()),
        ptest: PtestCapability::Configured,
    }
}

mod test_workflow_previews_exact_selftest_and_build_operations;

mod test_workflow_dialog_bounds_editing_and_reports_invalid_state;

mod test_workflow_rejects_missing_tools_bad_tokens_and_unconfigured_ptest;

fn result_identity(name: &str, fingerprint: &str) -> TestResultIdentity {
    TestResultIdentity::new(
        format!("/build/testresults/{name}/testresults.json").into(),
        1_024,
        SystemTime::UNIX_EPOCH,
        fingerprint.into(),
    )
    .unwrap()
}

fn case(suite: &str, name: &str, outcome: TestCaseOutcome) -> TestCaseRecord {
    TestCaseRecord::new(
        TestCaseIdentity::new(suite.into(), name.into()).unwrap(),
        outcome,
        Some(Duration::from_millis(10)),
        Vec::new(),
        Some(format!("/build/logs/{suite}-{name}.log").into()),
    )
    .unwrap()
    .0
}

fn result(name: &str, fingerprint: &str, cases: Vec<TestCaseRecord>) -> TestResultRecord {
    let (suite, _) = TestSuiteRecord::new("suite".into(), None, Vec::new(), cases).unwrap();
    TestResultRecord::new(
        result_identity(name, fingerprint),
        Some(TestFamily::TestImage),
        Some("qemux86-64".into()),
        Some("core-image-minimal".into()),
        Some("rev-1".into()),
        None,
        Vec::new(),
        vec![suite],
        Some(TestSessionId(1)),
        Vec::new(),
    )
    .0
}

mod test_results_normalize_exact_records_metadata_duplicates_and_bounds;

mod test_results_comparison_uses_exact_suite_case_status_transitions;

mod test_results_import_and_junit_require_exact_non_overwriting_paths;
