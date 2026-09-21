use super::*;

#[tokio::test]
async fn qa_report_preserves_empty_and_malformed_as_distinct_outcomes() {
    let root = TestDirectory::new();
    let empty = root.0.join("empty");
    fs::create_dir(&empty).unwrap();
    let scope = recipe_scope(&root.0);
    let check = QaCheckId::new("patch-busybox".into()).unwrap();
    let response = QaReportAdapter::new()
        .scan(input(
            &root.0,
            vec![candidate(empty, None, check.clone(), scope.clone())],
        ))
        .await
        .unwrap();
    assert!(matches!(response.outcome, QaReportScanOutcome::Empty));

    let malformed = root.0.join("malformed.json");
    fs::write(&malformed, "{").unwrap();
    assert!(matches!(
        QaReportAdapter::new()
            .scan(input(
                &root.0,
                vec![candidate(
                    malformed.clone(),
                    Some(QaReportFormat::Json),
                    check,
                    scope
                )],
            ))
            .await,
        Err(QaReportAdapterError::MalformedReport(path)) if path == malformed
    ));
}
