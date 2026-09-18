use super::*;

#[cfg(unix)]
#[tokio::test]
async fn test_workflow_cli_imports_compares_and_exports_exact_results() {
    let directory =
        std::env::temp_dir().join(format!("yoctui-testing-cli-results-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    let bin = directory.join("bin");
    let build = directory.join("build");
    let results = directory.join("results");
    let export = directory.join("export");
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(&build).unwrap();
    fs::create_dir_all(&results).unwrap();
    fs::create_dir_all(&export).unwrap();
    test_workflow_cli_executable(
        &bin.join("resulttool"),
        "#!/bin/sh\nif [ \"$1\" = junit ]; then : > \"$4\"; fi\nexit 0\n",
    );
    for tool in ["oe-selftest", "bitbake-selftest"] {
        test_workflow_cli_executable(&bin.join(tool), "#!/bin/sh\nexit 0\n");
    }
    let result_json = |status: &str| {
        format!(
            r#"{{"runtime":{{"configuration":{{"TEST_TYPE":"runtime","MACHINE":"qemux86-64","IMAGE_BASENAME":"core-image-minimal"}},"result":{{"runtime.Case.test_one":{{"status":"{status}"}}}}}}}}"#
        )
    };
    let baseline_path = results.join("baseline").join("testresults.json");
    let candidate_path = results.join("candidate").join("testresults.json");
    fs::create_dir_all(baseline_path.parent().unwrap()).unwrap();
    fs::create_dir_all(candidate_path.parent().unwrap()).unwrap();
    fs::write(&baseline_path, result_json("PASSED")).unwrap();
    fs::write(&candidate_path, result_json("FAILED")).unwrap();

    let mut app = App::new(100, 100_000);
    app.screen = Screen::Testing;
    let mut coordinator =
        TestCliCoordinator::new(build, vec![bin], yoctui_model::PtestCapability::Configured);
    let _ = coordinator
        .handle_effect(&mut app, Effect::InspectResultToolCapability)
        .await;
    let request =
        yoctui_model::TestResultImportRequest::new(1, vec![baseline_path, candidate_path]).unwrap();
    app.test_results = yoctui_model::TestResultInventoryState::Loading {
        request: request.clone(),
    };
    assert!(
        coordinator
            .handle_effect(&mut app, Effect::ImportTestResults(request))
            .await
    );
    for _ in 0..100 {
        coordinator.poll(&mut app).await;
        if app.test_results.records().len() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    assert_eq!(app.test_results.records().len(), 2);
    let baseline = app.test_results.records()[0].clone();
    let candidate = app.test_results.records()[1].clone();
    let request = yoctui_model::TestComparisonRequest::new(
        1,
        baseline.identity.clone(),
        candidate.identity.clone(),
    )
    .unwrap();
    app.test_comparison = yoctui_model::TestComparisonState::Loading {
        request: request.clone(),
    };
    assert!(
        coordinator
            .handle_effect(&mut app, Effect::CompareTestResults(request))
            .await
    );
    for _ in 0..100 {
        coordinator.poll(&mut app).await;
        if matches!(
            app.test_comparison,
            yoctui_model::TestComparisonState::Available { .. }
                | yoctui_model::TestComparisonState::Partial { .. }
        ) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    assert!(matches!(
        app.test_comparison,
        yoctui_model::TestComparisonState::Available { .. }
    ));

    let destination = export.join("results.xml");
    let inspection = coordinator
        .result_adapter
        .inspect_junit_destination(destination);
    let request =
        yoctui_model::TestJunitExportRequest::new(1, candidate.identity, &inspection).unwrap();
    app.test_junit_export = yoctui_model::TestJunitExportState::Running(request.clone());
    assert!(
        coordinator
            .handle_effect(&mut app, Effect::ExportTestJunit(request))
            .await
    );
    for _ in 0..100 {
        coordinator.poll(&mut app).await;
        if matches!(
            app.test_junit_export,
            yoctui_model::TestJunitExportState::Succeeded(_)
        ) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    assert!(matches!(
        app.test_junit_export,
        yoctui_model::TestJunitExportState::Succeeded(_)
    ));
    assert!(export.join("results.xml").is_file());
    fs::remove_dir_all(directory).unwrap();
}
