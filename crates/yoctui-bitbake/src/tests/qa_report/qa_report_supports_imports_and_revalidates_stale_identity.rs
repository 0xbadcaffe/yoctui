use super::*;

#[tokio::test]
async fn qa_report_supports_imports_and_revalidates_stale_identity() {
    let build = TestDirectory::new();
    let imported = TestDirectory::new();
    let scope = recipe_scope(&build.0);
    let check = QaCheckId::new("recipe-package-busybox".into()).unwrap();
    let report = imported.0.join("import.json");
    fs::write(
        &report,
        r#"[{"status":"passed","message":"package QA passed"}]"#,
    )
    .unwrap();
    let mut exact = candidate(report.clone(), Some(QaReportFormat::Json), check, scope);
    exact.origin = QaReportOrigin::Import;
    let response = QaReportAdapter::new()
        .scan(input(&build.0, vec![exact]))
        .await
        .unwrap();
    let identity = response.outcome.reports()[0].identity.clone();
    QaReportAdapter::new().revalidate(&identity).unwrap();
    fs::write(&report, r#"[{"status":"failed","message":"changed"}]"#).unwrap();
    assert!(matches!(
        QaReportAdapter::new().revalidate(&identity),
        Err(QaReportAdapterError::StaleReport(path)) if path == report
    ));
}
