use super::*;

#[tokio::test]
async fn qa_report_rejects_missing_symlink_escape_duplicate_and_mismatch() {
    let root = TestDirectory::new();
    let scope = recipe_scope(&root.0);
    let check = QaCheckId::new("license-busybox".into()).unwrap();
    let missing = root.0.join("missing.json");
    assert!(matches!(
        QaReportAdapter::new()
            .scan(input(
                &root.0,
                vec![candidate(
                    missing.clone(),
                    Some(QaReportFormat::Json),
                    check.clone(),
                    scope.clone()
                )],
            ))
            .await,
        Err(QaReportAdapterError::MissingPath(path)) if path == missing
    ));

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let real = root.0.join("real.json");
        let link = root.0.join("link.json");
        fs::write(&real, "[]").unwrap();
        symlink(&real, &link).unwrap();
        assert!(matches!(
            QaReportAdapter::new()
                .scan(input(
                    &root.0,
                    vec![candidate(
                        link.clone(),
                        Some(QaReportFormat::Json),
                        check.clone(),
                        scope.clone()
                    )],
                ))
                .await,
            Err(QaReportAdapterError::SymlinkPath(path)) if path == link
        ));
    }

    let outside = TestDirectory::new();
    let escaped = outside.0.join("report.json");
    fs::write(&escaped, "[]").unwrap();
    assert!(matches!(
        QaReportAdapter::new()
            .scan(input(
                &root.0,
                vec![candidate(
                    escaped.clone(),
                    Some(QaReportFormat::Json),
                    check.clone(),
                    scope.clone()
                )],
            ))
            .await,
        Err(QaReportAdapterError::EscapePath(path)) if path == escaped
    ));

    let valid = root.0.join("valid.json");
    fs::write(&valid, "[]").unwrap();
    let duplicate = candidate(valid.clone(), Some(QaReportFormat::Json), check, scope);
    let mut bad = input(&root.0, vec![duplicate.clone()]);
    bad.candidates.push(duplicate);
    assert!(matches!(
        QaReportAdapter::new().scan(bad).await,
        Err(QaReportAdapterError::InvalidRequest(_))
    ));
}
