include!("qa_layer/capability_and_validation.rs");

include!("qa_layer/command_spec.rs");

include!("qa_layer/job_runner.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-qa-layer-{name}-{}-{}",
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

    #[cfg(unix)]
    fn write_executable(path: &Path, body: &str) {
        crate::test_support::write_executable(path, body);
    }

    #[cfg(unix)]
    fn fixture(name: &str, body: &str) -> (TestDirectory, QaLayerCapabilitySnapshot) {
        let root = TestDirectory::new(name);
        let bin = root.0.join("bin");
        let layer = root.0.join("meta-demo");
        let reports = root.0.join("reports");
        fs::create_dir(&bin).unwrap();
        fs::create_dir(&layer).unwrap();
        fs::create_dir(&reports).unwrap();
        write_executable(&bin.join("yocto-check-layer"), body);
        let identity = QaLayerIdentity::new("meta-demo".into(), layer).unwrap();
        let input = QaLayerCapabilityInput {
            release: Some("6.0".into()),
            build_directory: root.0.clone(),
            selected_layer: identity.clone(),
            layers: vec![QaConfiguredLayerInput {
                check: QaCheckId::new("layer-meta-demo".into()).unwrap(),
                identity,
                compatible_series: vec!["walnascar".into()],
                report_roots: vec![reports],
            }],
            executable_search_path: vec![bin],
        };
        let response = QaLayerCapabilityInspector::inspect(input).unwrap();
        let snapshot = match response {
            QaLayerCapabilityResponse::Available(snapshot) => snapshot,
            QaLayerCapabilityResponse::Partial(_) => panic!("expected complete capability"),
        };
        (root, snapshot)
    }

    fn preview(snapshot: &QaLayerCapabilitySnapshot) -> QaLayerOperationPreview {
        let layer = &snapshot.layers[0];
        let QaLayerRunCapability::Available {
            executable,
            arguments,
            report_roots,
        } = &layer.run
        else {
            panic!("expected runnable layer");
        };
        QaLayerOperationPreview {
            id: yoctui_model::QaLayerOperationId(4),
            check: layer.check.clone(),
            layer: layer.identity.clone(),
            executable: executable.clone(),
            arguments: arguments.clone(),
            indexed_arguments: indexed_arguments(&executable.path, arguments),
            report_roots: report_roots.clone(),
            limitations: Vec::new(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn qa_layer_capability_discovers_exact_configured_layers_and_partial_inputs() {
        let (root, snapshot) = fixture("capability", "#!/bin/sh\nexit 0\n");
        assert_eq!(snapshot.layers.len(), 1);
        assert!(matches!(
            snapshot.layers[0].run,
            QaLayerRunCapability::Available { .. }
        ));
        let missing = root.0.join("missing-bin");
        fs::create_dir(&missing).unwrap();
        let identity = snapshot.selected_layer.clone();
        let response = QaLayerCapabilityInspector::inspect(QaLayerCapabilityInput {
            release: None,
            build_directory: root.0.clone(),
            selected_layer: identity.clone(),
            layers: vec![QaConfiguredLayerInput {
                check: QaCheckId::new("layer-meta-demo".into()).unwrap(),
                identity,
                compatible_series: Vec::new(),
                report_roots: Vec::new(),
            }],
            executable_search_path: vec![missing],
        })
        .unwrap();
        assert!(matches!(response, QaLayerCapabilityResponse::Partial(_)));
    }

    #[cfg(unix)]
    #[test]
    fn qa_layer_capability_and_command_reject_symlink_tampering_and_preview_changes() {
        let (root, snapshot) = fixture("safety", "#!/bin/sh\nexit 0\n");
        let mut exact = preview(&snapshot);
        let command = QaLayerCommandSpec::from_preview(QaLayerSessionId(8), &exact).unwrap();
        assert_eq!(
            command.arguments(),
            &[OsString::from(exact.layer.root.as_os_str())]
        );
        exact.indexed_arguments.push("2: injected".into());
        assert!(matches!(
            QaLayerCommandSpec::from_preview(QaLayerSessionId(8), &exact),
            Err(QaLayerAdapterError::PreviewMismatch)
        ));
        let executable = command.executable().to_owned();
        write_executable(&executable, "#!/bin/sh\nexit 1\n");
        assert!(matches!(
            command.revalidate(),
            Err(QaLayerAdapterError::StaleIdentity(path)) if path == executable
        ));

        let real = root.0.join("real-bin");
        fs::create_dir(&real).unwrap();
        write_executable(&real.join("yocto-check-layer"), "#!/bin/sh\nexit 0\n");
        let link = root.0.join("linked-bin");
        symlink(&real, &link).unwrap();
        let mut limitations = Vec::new();
        assert!(discover_executable(&[link], &mut limitations).is_none());
        assert!(!limitations.is_empty());
    }

    #[cfg(unix)]
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
            .start(
                QaLayerCommandSpec::from_preview(QaLayerSessionId(10), &preview(&snapshot))
                    .unwrap(),
            )
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

    #[cfg(unix)]
    #[tokio::test]
    async fn qa_layer_runner_rejects_duplicate_and_cancels_gracefully_or_forcibly() {
        let (_root, snapshot) = fixture(
            "cancel",
            "#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n",
        );
        let command =
            QaLayerCommandSpec::from_preview(QaLayerSessionId(11), &preview(&snapshot)).unwrap();
        let mut runner = QaLayerJobRunner::new();
        runner.start(command.clone()).await.unwrap();
        assert!(matches!(
            runner.start(command).await,
            Err(QaLayerAdapterError::Busy)
        ));
        assert!(runner.cancel(QaLayerSessionId(11)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::CancellationRequested { .. }
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::Cancelled { forced: false, .. }
        ));

        let (_root, snapshot) = fixture(
            "forced",
            "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
        );
        let command =
            QaLayerCommandSpec::from_preview(QaLayerSessionId(12), &preview(&snapshot)).unwrap();
        let mut runner =
            QaLayerJobRunner::new().with_cancellation_timeout(Duration::from_millis(10));
        runner.start(command).await.unwrap();
        runner.next_event().await.unwrap();
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert!(runner.cancel(QaLayerSessionId(12)).await.unwrap());
        runner.next_event().await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::Cancelled { forced: true, .. }
        ));
    }

    #[cfg(unix)]
    #[test]
    fn qa_layer_spawn_retry_classifies_only_text_file_busy_as_transient() {
        assert!(is_transient_qa_layer_spawn_error(
            &io::Error::from_raw_os_error(libc::ETXTBSY,)
        ));
        assert!(!is_transient_qa_layer_spawn_error(
            &io::Error::from_raw_os_error(libc::ENOENT),
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qa_layer_runner_preserves_timeout_rejection_and_channel_loss() {
        let (_root, snapshot) = fixture(
            "terminal",
            "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
        );
        let command =
            QaLayerCommandSpec::from_preview(QaLayerSessionId(13), &preview(&snapshot)).unwrap();
        let mut runner = QaLayerJobRunner::new()
            .with_operation_timeout(Duration::from_millis(1))
            .with_cancellation_timeout(Duration::from_millis(1));
        runner.start(command).await.unwrap();
        runner.next_event().await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::TimedOut { .. }
        ));
        assert!(!runner.cancel(QaLayerSessionId(99)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::CancellationRejected { .. }
        ));

        let (_root, snapshot) = fixture("loss", "#!/bin/sh\nsleep 30\n");
        let command =
            QaLayerCommandSpec::from_preview(QaLayerSessionId(14), &preview(&snapshot)).unwrap();
        let mut runner = QaLayerJobRunner::new();
        runner.start(command).await.unwrap();
        runner.next_event().await.unwrap();
        runner.lose_output_channel();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            QaLayerRunnerEvent::Lost { .. }
        ));
    }
}
