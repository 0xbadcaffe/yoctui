use super::*;

#[test]
fn test_results_normalize_exact_records_metadata_duplicates_and_bounds() {
    let duplicate = case("suite", "same", TestCaseOutcome::Passed);
    let (suite, suite_limitations) = TestSuiteRecord::new(
        "suite".into(),
        None,
        vec![
            TestMetadata::new("z".into(), "last".into()).unwrap(),
            TestMetadata::new("z".into(), "ignored".into()).unwrap(),
            TestMetadata::new("a".into(), "first".into()).unwrap(),
        ],
        vec![duplicate.clone(), duplicate],
    )
    .unwrap();
    assert_eq!(suite.metadata[0].key, "a");
    assert_eq!(suite.cases.len(), 1);
    assert!(
        suite_limitations
            .iter()
            .any(|value| value.contains("duplicate"))
    );

    let record = TestResultRecord::new(
        result_identity("one", "abc123"),
        None,
        Some("\n".into()),
        None,
        None,
        None,
        Vec::new(),
        vec![suite],
        None,
        vec!["adapter skipped malformed record".into()],
    )
    .0;
    assert_eq!(record.machine, None);
    assert_eq!(record.counts().passed, 1);
    assert!(
        record
            .limitations
            .iter()
            .any(|value| value.contains("invalid machine"))
    );

    let (records, limitations) = normalize_test_results(
        vec![record.clone(), record],
        vec!["adapter skipped malformed record".into()],
    );
    assert_eq!(records.len(), 1);
    assert!(
        limitations
            .iter()
            .any(|value| value.contains("duplicate exact test results"))
    );
    assert!(
        TestResultIdentity::new(
            "relative.json".into(),
            1,
            SystemTime::UNIX_EPOCH,
            "abc".into()
        )
        .is_err()
    );
}
