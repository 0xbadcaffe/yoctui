use super::*;

#[tokio::test]
async fn qa_layer_runner_streams_bounded_output_and_reports_success_and_nonzero() {
    let (root, snapshot) = fixture(
        "runner",
        &format!(
            "#!/bin/sh\nprintf 'out\\n'\nprintf '%*s\\n' {} x >&2\nexit 0\n",
            MAX_QA_TEXT_BYTES + 64
        ),
    );
    let command =
        QaLayerCommandSpec::from_preview(QaLayerSessionId(9), &preview(&snapshot)).unwrap();
    let mut runner = QaLayerJobRunner::new();
    runner.start(command).await.unwrap();
    assert_eq!(
        runner.next_event().await.unwrap(),
        QaLayerRunnerEvent::Started {
            id: QaLayerSessionId(9)
        }
    );
    let mut saw_truncated = false;
    loop {
        match runner.next_event().await.unwrap() {
            QaLayerRunnerEvent::Output { truncated, .. } => saw_truncated |= truncated,
            QaLayerRunnerEvent::Completed { exit_code, .. } => {
                assert_eq!(exit_code, Some(0));
                break;
            }
            event => panic!("unexpected event: {event:?}"),
        }
    }
    assert!(saw_truncated);

    let executable = root.0.join("bin/yocto-check-layer");
    write_executable(&executable, "#!/bin/sh\nexit 7\n");
    let response = QaLayerCapabilityInspector::inspect(QaLayerCapabilityInput {
        release: None,
        build_directory: root.0.clone(),
        selected_layer: snapshot.selected_layer.clone(),
        layers: vec![QaConfiguredLayerInput {
            check: snapshot.layers[0].check.clone(),
            identity: snapshot.selected_layer.clone(),
            compatible_series: Vec::new(),
            report_roots: Vec::new(),
        }],
        executable_search_path: vec![root.0.join("bin")],
    })
    .unwrap();
    let QaLayerCapabilityResponse::Available(snapshot) = response else {
        panic!("expected capability");
    };
    let mut runner = QaLayerJobRunner::new();
    runner
        .start(QaLayerCommandSpec::from_preview(QaLayerSessionId(10), &preview(&snapshot)).unwrap())
        .await
        .unwrap();
    assert!(matches!(
        runner.next_event().await.unwrap(),
        QaLayerRunnerEvent::Started { .. }
    ));
    assert!(matches!(
        runner.next_event().await.unwrap(),
        QaLayerRunnerEvent::Failed {
            exit_code: Some(7),
            ..
        }
    ));
}
