include!("qa_report/types_and_adapter.rs");

include!("qa_report/report_acquisition.rs");

include!("qa_report/json_parsing.rs");

include!("qa_report/text_and_xml_parsing.rs");

include!("qa_report/metadata_and_limits.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use yoctui_model::{QaLayerIdentity, QaScope, RecipeIdentity};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-qa-report-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn recipe_scope(root: &Path) -> QaFindingScope {
        let provider = root.join("busybox.bb");
        fs::write(&provider, "SUMMARY = \"BusyBox\"\n").unwrap();
        QaFindingScope::Recipe(
            QaScope::new(RecipeIdentity {
                name: "busybox".into(),
                file: provider,
            })
            .unwrap(),
        )
    }

    fn layer_scope(root: &Path) -> QaFindingScope {
        let layer = root.join("meta-demo");
        fs::create_dir_all(&layer).unwrap();
        QaFindingScope::Layer(QaLayerIdentity::new("meta-demo".into(), layer).unwrap())
    }

    fn candidate(
        path: PathBuf,
        format: Option<QaReportFormat>,
        producer: QaCheckId,
        scope: QaFindingScope,
    ) -> QaReportCandidate {
        let (task, test_name) = match scope {
            QaFindingScope::Recipe(_) => (Some("do_package_qa".into()), None),
            QaFindingScope::Layer(_) => (None, Some("bsp.test".into())),
        };
        QaReportCandidate {
            path,
            origin: QaReportOrigin::Managed,
            format,
            producer,
            scope,
            task,
            test_name,
        }
    }

    fn input(root: &Path, candidates: Vec<QaReportCandidate>) -> QaReportScanInput {
        let mut paths = candidates
            .iter()
            .map(|candidate| candidate.path.clone())
            .collect::<Vec<_>>();
        paths.sort();
        let mut checks = candidates
            .iter()
            .map(|candidate| candidate.producer.clone())
            .collect::<Vec<_>>();
        checks.sort();
        checks.dedup();
        let mut scopes = candidates
            .iter()
            .map(|candidate| candidate.scope.clone())
            .collect::<Vec<_>>();
        scopes.sort_by_key(|scope| format!("{scope:?}"));
        scopes.dedup();
        QaReportScanInput {
            build_directory: root.into(),
            request: QaReportRequest::new(1, paths).unwrap(),
            candidates,
            known_checks: checks,
            known_scopes: scopes,
        }
    }

    #[tokio::test]
    async fn qa_report_parses_exact_json_text_xml_and_bitbake_records() {
        let root = TestDirectory::new();
        let recipe = recipe_scope(&root.0);
        let layer = layer_scope(&root.0);
        let check = QaCheckId::new("recipe-package-busybox".into()).unwrap();
        let layer_check = QaCheckId::new("layer-meta-demo".into()).unwrap();
        let source = root.0.join("source.bbclass");
        fs::write(&source, "# source\n").unwrap();
        let json = root.0.join("report.json");
        fs::write(
            &json,
            format!(
                r#"{{"metadata":{{"tool":"oeqa"}},"findings":[{{"status":"failed","severity":"error","message":"license checksum mismatch","rule":"license-checksum","suggestion":"refresh LIC_FILES_CHKSUM","source":{{"path":"{}","line":7}},"metadata":{{"package":"busybox"}}}}]}}"#,
                source.display()
            ),
        )
        .unwrap();
        let text = root.0.join("report.qa");
        fs::write(
            &text,
            "warning\tpatch has fuzz\tseverity=warning\trule=patch-fuzz\n",
        )
        .unwrap();
        let xml = root.0.join("report.xml");
        fs::write(
            &xml,
            r#"<qa-report><finding status="passed" message="layer compatible" rule="compat"/></qa-report>"#,
        )
        .unwrap();
        let log = root.0.join("log.do_package_qa.log");
        fs::write(
            &log,
            "NOTE: ignored\nERROR: QA Issue: installed-vs-shipped mismatch [installed-vs-shipped]\n",
        )
        .unwrap();
        let response = QaReportAdapter::new()
            .scan(input(
                &root.0,
                vec![
                    candidate(
                        json,
                        Some(QaReportFormat::Json),
                        check.clone(),
                        recipe.clone(),
                    ),
                    candidate(
                        text,
                        Some(QaReportFormat::Text),
                        check.clone(),
                        recipe.clone(),
                    ),
                    candidate(xml, Some(QaReportFormat::Xml), layer_check, layer),
                    candidate(log, Some(QaReportFormat::BitBakeLog), check, recipe),
                ],
            ))
            .await
            .unwrap();
        assert_eq!(response.outcome.reports().len(), 4);
        assert_eq!(
            response
                .outcome
                .reports()
                .iter()
                .map(|report| report.findings.len())
                .sum::<usize>(),
            4
        );
        assert!(
            response
                .outcome
                .limitations()
                .iter()
                .any(|value| value.contains("record 1"))
        );
        assert_eq!(
            response.outcome.reports()[0].identity.producer,
            Some(QaCheckId("recipe-package-busybox".into()))
        );
    }

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
}
