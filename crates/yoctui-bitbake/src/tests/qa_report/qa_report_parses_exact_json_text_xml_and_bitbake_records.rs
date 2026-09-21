use super::*;

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
