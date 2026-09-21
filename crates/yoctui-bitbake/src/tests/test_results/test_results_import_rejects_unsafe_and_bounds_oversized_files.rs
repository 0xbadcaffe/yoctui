use super::*;

#[test]
fn test_results_import_rejects_unsafe_and_bounds_oversized_files() {
    let (directory, adapter, _tool, results) = fixture("bounds");
    let oversized = results.join("testresults.json");
    let file = fs::File::create(&oversized).unwrap();
    file.set_len(MAX_RESULT_FILE_BYTES + 1).unwrap();
    let response = imported(&adapter, 1, vec![oversized]);
    assert!(response.records.is_empty());
    assert_eq!(response.limitations.len(), 1);
    assert!(response.limitations[0].contains("invalid byte size"));

    let wrong_name = directory.path().join("arbitrary.json");
    fs::write(&wrong_name, result_json("PASSED")).unwrap();
    let request = TestResultImportRequest::new(2, vec![wrong_name]).unwrap();
    assert!(matches!(
        adapter.import(&request),
        Err(TestResultAdapterError::UnsafeResult(_))
    ));
}
