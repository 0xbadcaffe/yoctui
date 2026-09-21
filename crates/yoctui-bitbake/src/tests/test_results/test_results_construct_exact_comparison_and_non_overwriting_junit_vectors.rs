use super::*;

#[test]
fn test_results_construct_exact_comparison_and_non_overwriting_junit_vectors() {
    let (directory, adapter, tool, results) = fixture("vectors");
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
    let baseline = response
        .records
        .iter()
        .find(|record| record.identity.path == baseline_path)
        .unwrap();
    let candidate = response
        .records
        .iter()
        .find(|record| record.identity.path == candidate_path)
        .unwrap();
    let comparison = TestComparison::between(baseline, candidate).unwrap();
    assert_eq!(comparison.baseline, baseline.identity);
    assert_eq!(comparison.candidate, candidate.identity);
    let request =
        TestComparisonRequest::new(4, baseline.identity.clone(), candidate.identity.clone())
            .unwrap();
    let preview = TestComparisonPreview::new(tool.clone(), request.clone()).unwrap();
    let command = adapter
        .comparison_command(&preview, baseline, candidate)
        .unwrap();
    assert_eq!(
        command.arguments(),
        [
            OsStr::new("regression-file"),
            baseline_path.as_os_str(),
            candidate_path.as_os_str()
        ]
    );
    assert_eq!(
        command.operation(),
        &TestResultOperation::Comparison(request)
    );

    let export_directory = directory.path().join("export");
    fs::create_dir(&export_directory).unwrap();
    let destination = export_directory.join("results.xml");
    let inspection = TestJunitDestinationInspection {
        requested: destination.clone(),
        canonical_parent: Some(export_directory),
        parent_exists: true,
        parent_is_directory: true,
        destination_exists: false,
        destination_is_symlink: false,
    };
    let request = TestJunitExportRequest::new(5, candidate.identity.clone(), &inspection).unwrap();
    let preview = TestJunitExportPreview::new(tool, request.clone()).unwrap();
    let command = adapter.junit_command(&preview, candidate).unwrap();
    assert_eq!(
        command.arguments(),
        [
            OsStr::new("junit"),
            candidate_path.as_os_str(),
            OsStr::new("-j"),
            destination.as_os_str()
        ]
    );
    fs::write(&destination, b"do not overwrite").unwrap();
    assert!(matches!(
        command.revalidate(),
        Err(TestResultAdapterError::UnsafeDestination(_))
    ));
}
