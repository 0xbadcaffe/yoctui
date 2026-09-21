use super::*;

#[tokio::test]
async fn qa_report_preserves_cancellation_timeout_loss_and_oversize() {
    let root = TestDirectory::new();
    let scope = recipe_scope(&root.0);
    let check = QaCheckId::new("recipe-package-busybox".into()).unwrap();
    let report = root.0.join("report.json");
    fs::write(&report, "[]").unwrap();
    let scan = input(
        &root.0,
        vec![candidate(
            report.clone(),
            Some(QaReportFormat::Json),
            check.clone(),
            scope.clone(),
        )],
    );
    let cancellation = QaReportCancellation::default();
    cancellation.cancel();
    assert!(matches!(
        QaReportAdapter::new()
            .scan_with_cancellation(scan.clone(), cancellation)
            .await,
        Err(QaReportAdapterError::Cancelled)
    ));
    assert!(matches!(
        QaReportAdapter::new()
            .with_timeout(Duration::ZERO)
            .scan(scan.clone())
            .await,
        Err(QaReportAdapterError::Timeout(0))
    ));
    assert!(matches!(
        QaReportAdapter::new().with_worker_panic().scan(scan).await,
        Err(QaReportAdapterError::WorkerLost(_))
    ));

    let oversized = root.0.join("large.json");
    fs::File::create(&oversized)
        .unwrap()
        .set_len(MAX_QA_FILE_BYTES + 1)
        .unwrap();
    assert!(matches!(
        QaReportAdapter::new()
            .scan(input(
                &root.0,
                vec![candidate(
                    oversized.clone(),
                    Some(QaReportFormat::Json),
                    check,
                    scope
                )],
            ))
            .await,
        Err(QaReportAdapterError::OversizedReport(path)) if path == oversized
    ));
}
