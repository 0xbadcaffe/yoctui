use super::*;

#[tokio::test]
async fn qa_report_directory_scan_is_bounded_exact_and_partial() {
    let root = TestDirectory::new();
    let reports = root.0.join("reports");
    fs::create_dir(&reports).unwrap();
    fs::write(
        reports.join("valid.json"),
        r#"[{"status":"passed","message":"URI is reachable"}]"#,
    )
    .unwrap();
    fs::write(reports.join("bad.json"), "{").unwrap();
    fs::write(reports.join("ignored.bin"), "not a report").unwrap();
    let scope = recipe_scope(&root.0);
    let check = QaCheckId::new("uri-fetch-busybox".into()).unwrap();
    let response = QaReportAdapter::new()
        .scan(input(&root.0, vec![candidate(reports, None, check, scope)]))
        .await
        .unwrap();
    assert_eq!(response.outcome.reports().len(), 1);
    assert!(matches!(
        response.outcome,
        QaReportScanOutcome::Partial { .. }
    ));
    assert!(
        response
            .outcome
            .limitations()
            .iter()
            .any(|value| value.contains("unsupported"))
    );
}
