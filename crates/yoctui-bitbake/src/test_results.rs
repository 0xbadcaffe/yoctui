include!("test_results/adapter_and_capability.rs");

include!("test_results/commands_and_identity.rs");

include!("test_results/job_runner.rs");

include!("test_results/result_discovery_and_parsing.rs");

include!("test_results/case_and_path_validation.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        ffi::OsStr,
        sync::atomic::{AtomicU64, Ordering},
    };
    use yoctui_model::{TestComparison, TestJunitDestinationInspection, TestResultInventoryState};

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-test-results-{name}-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn executable(path: &Path, body: &str) {
        crate::test_support::write_executable(path, body);
    }

    fn result_json(status: &str) -> String {
        format!(
            r#"{{
                "runtime-qemu": {{
                    "configuration": {{
                        "TEST_TYPE": "runtime",
                        "MACHINE": "qemux86-64",
                        "IMAGE_BASENAME": "core-image-minimal",
                        "OECOREREV": "abc123"
                    }},
                    "result": {{
                        "runtime.Case.test_one": {{
                            "status": "{status}",
                            "duration": 1.25,
                            "log": "bounded diagnostic"
                        }}
                    }}
                }}
            }}"#
        )
    }

    fn fixture(name: &str) -> (TestDirectory, TestResultAdapter, PathBuf, PathBuf) {
        let directory = TestDirectory::new(name);
        let bin = directory.path().join("bin");
        let results = directory.path().join("results");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(&results).unwrap();
        let resulttool = bin.join("resulttool");
        executable(&resulttool, "#!/bin/sh\nprintf '%s\\n' \"$@\"\nexit 0\n");
        let adapter = TestResultAdapter::new(vec![bin]);
        (directory, adapter, resulttool, results)
    }

    fn imported(
        adapter: &TestResultAdapter,
        generation: u64,
        paths: Vec<PathBuf>,
    ) -> TestResultImportResponse {
        adapter
            .import(&TestResultImportRequest::new(generation, paths).unwrap())
            .unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn test_results_capability_is_independent_canonical_and_fail_closed() {
        let directory = TestDirectory::new("capability");
        let bin = directory.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let inspector = ResultToolCapabilityInspector::new(vec![bin.clone()]);
        assert_eq!(inspector.inspect(), ResultToolCapability::Missing);

        let tool = bin.join("resulttool");
        executable(&tool, "#!/bin/sh\nexit 0\n");
        assert_eq!(
            inspector.inspect(),
            ResultToolCapability::Available(tool.clone())
        );

        fs::remove_file(&tool).unwrap();
        let outside = directory.path().join("outside-resulttool");
        executable(&outside, "#!/bin/sh\nexit 0\n");
        symlink(&outside, &tool).unwrap();
        assert!(matches!(
            inspector.inspect(),
            ResultToolCapability::Failed(_)
        ));
        assert!(matches!(
            ResultToolCapabilityInspector::new(vec![directory.path().join("missing")]).inspect(),
            ResultToolCapability::Failed(_)
        ));
    }

    #[test]
    fn test_results_imports_official_shape_and_preserves_partial_failures() {
        let (_directory, adapter, _tool, results) = fixture("import");
        let valid = results.join("valid").join("testresults.json");
        let malformed = results.join("malformed").join("testresults.json");
        let empty = results.join("empty").join("testresults.json");
        for path in [&valid, &malformed, &empty] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
        }
        fs::write(&valid, result_json("PASSED")).unwrap();
        fs::write(&malformed, b"{not json").unwrap();
        fs::write(&empty, b"{}").unwrap();

        let response = imported(&adapter, 7, vec![results]);
        assert_eq!(response.records.len(), 1);
        let record = &response.records[0];
        assert_eq!(record.family, Some(TestFamily::TestImage));
        assert_eq!(record.machine.as_deref(), Some("qemux86-64"));
        assert_eq!(record.image.as_deref(), Some("core-image-minimal"));
        assert_eq!(record.revision.as_deref(), Some("abc123"));
        assert_eq!(record.counts().passed, 1);
        assert_eq!(record.identity.fingerprint.len(), 64);
        assert!(record.is_valid());
        assert_eq!(response.limitations.len(), 2);
        assert!(
            response
                .limitations
                .iter()
                .any(|message| message.contains("malformed"))
        );
        assert!(
            response
                .limitations
                .iter()
                .any(|message| message.contains("no typed result runs"))
        );
        let state = if response.limitations.is_empty() {
            TestResultInventoryState::Available {
                request: response.request,
                records: response.records,
            }
        } else {
            TestResultInventoryState::Partial {
                request: response.request,
                records: response.records,
                limitations: response.limitations,
            }
        };
        assert!(matches!(state, TestResultInventoryState::Partial { .. }));
    }

    #[test]
    fn test_results_import_rejects_unsafe_and_bounds_oversized_files() {
        let (directory, adapter, _tool, results) = fixture("bounds");
        let oversized = results.join("testresults.json");
        let file = fs::File::create(&oversized).unwrap();
        file.set_len(MAX_RESULT_FILE_BYTES + 1).unwrap();
        let response = imported(&adapter, 1, vec![oversized]);
        assert!(response.records.is_empty());
        assert_eq!(response.limitations.len(), 1);
        assert!(response.limitations[0].contains("invalid byte size"));

        let wrong_name = directory.path().join("arbitrary.json");
        fs::write(&wrong_name, result_json("PASSED")).unwrap();
        let request = TestResultImportRequest::new(2, vec![wrong_name]).unwrap();
        assert!(matches!(
            adapter.import(&request),
            Err(TestResultAdapterError::UnsafeResult(_))
        ));
    }

    #[test]
    fn test_results_construct_exact_comparison_and_non_overwriting_junit_vectors() {
        let (directory, adapter, tool, results) = fixture("vectors");
        let baseline_path = results.join("baseline").join("testresults.json");
        let candidate_path = results.join("candidate").join("testresults.json");
        for path in [&baseline_path, &candidate_path] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
        }
        fs::write(&baseline_path, result_json("PASSED")).unwrap();
        fs::write(&candidate_path, result_json("FAILED")).unwrap();
        let response = imported(
            &adapter,
            1,
            vec![baseline_path.clone(), candidate_path.clone()],
        );
        let baseline = response
            .records
            .iter()
            .find(|record| record.identity.path == baseline_path)
            .unwrap();
        let candidate = response
            .records
            .iter()
            .find(|record| record.identity.path == candidate_path)
            .unwrap();
        let comparison = TestComparison::between(baseline, candidate).unwrap();
        assert_eq!(comparison.baseline, baseline.identity);
        assert_eq!(comparison.candidate, candidate.identity);
        let request =
            TestComparisonRequest::new(4, baseline.identity.clone(), candidate.identity.clone())
                .unwrap();
        let preview = TestComparisonPreview::new(tool.clone(), request.clone()).unwrap();
        let command = adapter
            .comparison_command(&preview, baseline, candidate)
            .unwrap();
        assert_eq!(
            command.arguments(),
            [
                OsStr::new("regression-file"),
                baseline_path.as_os_str(),
                candidate_path.as_os_str()
            ]
        );
        assert_eq!(
            command.operation(),
            &TestResultOperation::Comparison(request)
        );

        let export_directory = directory.path().join("export");
        fs::create_dir(&export_directory).unwrap();
        let destination = export_directory.join("results.xml");
        let inspection = TestJunitDestinationInspection {
            requested: destination.clone(),
            canonical_parent: Some(export_directory),
            parent_exists: true,
            parent_is_directory: true,
            destination_exists: false,
            destination_is_symlink: false,
        };
        let request =
            TestJunitExportRequest::new(5, candidate.identity.clone(), &inspection).unwrap();
        let preview = TestJunitExportPreview::new(tool, request.clone()).unwrap();
        let command = adapter.junit_command(&preview, candidate).unwrap();
        assert_eq!(
            command.arguments(),
            [
                OsStr::new("junit"),
                candidate_path.as_os_str(),
                OsStr::new("-j"),
                destination.as_os_str()
            ]
        );
        fs::write(&destination, b"do not overwrite").unwrap();
        assert!(matches!(
            command.revalidate(),
            Err(TestResultAdapterError::UnsafeDestination(_))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_results_revalidate_tampering_stream_output_and_nonzero() {
        let (_directory, adapter, tool, results) = fixture("runner");
        let baseline_path = results.join("baseline").join("testresults.json");
        let candidate_path = results.join("candidate").join("testresults.json");
        for path in [&baseline_path, &candidate_path] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
        }
        fs::write(&baseline_path, result_json("PASSED")).unwrap();
        fs::write(&candidate_path, result_json("FAILED")).unwrap();
        let response = imported(
            &adapter,
            1,
            vec![baseline_path.clone(), candidate_path.clone()],
        );
        let baseline = &response.records[0];
        let candidate = &response.records[1];
        let request =
            TestComparisonRequest::new(1, baseline.identity.clone(), candidate.identity.clone())
                .unwrap();
        let preview = TestComparisonPreview::new(tool.clone(), request.clone()).unwrap();
        let stale = adapter
            .comparison_command(&preview, baseline, candidate)
            .unwrap();
        fs::write(&candidate.identity.path, result_json("ERROR")).unwrap();
        assert!(matches!(
            TestResultJob::new().start(stale).await,
            Err(TestResultAdapterError::StaleResult(_))
        ));

        let response = imported(
            &adapter,
            2,
            vec![baseline_path.clone(), candidate_path.clone()],
        );
        let baseline = &response.records[0];
        let candidate = &response.records[1];
        let request =
            TestComparisonRequest::new(2, baseline.identity.clone(), candidate.identity.clone())
                .unwrap();
        executable(
            &tool,
            "#!/bin/sh\nprintf 'stdout\\n'\nprintf 'stderr\\n' >&2\nexit 7\n",
        );
        let preview = TestComparisonPreview::new(tool, request.clone()).unwrap();
        let command = adapter
            .comparison_command(&preview, baseline, candidate)
            .unwrap();
        let mut runner = TestResultJob::new();
        runner.start(command.clone()).await.unwrap();
        assert_eq!(
            runner.start(command).await,
            Err(TestResultAdapterError::Busy)
        );
        assert_eq!(
            runner.next_event().await.unwrap(),
            TestResultRunnerEvent::Started {
                operation: TestResultOperation::Comparison(request.clone())
            }
        );
        let mut stdout = false;
        let mut stderr = false;
        loop {
            match runner.next_event().await.unwrap() {
                TestResultRunnerEvent::Output { stream, .. } => {
                    stdout |= stream == TestOutputStream::Stdout;
                    stderr |= stream == TestOutputStream::Stderr;
                }
                TestResultRunnerEvent::Failed {
                    operation,
                    exit_code,
                } => {
                    assert_eq!(operation, TestResultOperation::Comparison(request));
                    assert_eq!(exit_code, Some(7));
                    break;
                }
                event => panic!("unexpected resulttool event: {event:?}"),
            }
        }
        assert!(stdout && stderr);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_results_report_cancellation_timeout_and_worker_loss() {
        let (_directory, adapter, tool, results) = fixture("control");
        let baseline_path = results.join("baseline").join("testresults.json");
        let candidate_path = results.join("candidate").join("testresults.json");
        for path in [&baseline_path, &candidate_path] {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
        }
        fs::write(&baseline_path, result_json("PASSED")).unwrap();
        fs::write(&candidate_path, result_json("FAILED")).unwrap();
        let response = imported(&adapter, 1, vec![baseline_path, candidate_path]);
        let baseline = &response.records[0];
        let candidate = &response.records[1];
        let request =
            TestComparisonRequest::new(1, baseline.identity.clone(), candidate.identity.clone())
                .unwrap();

        executable(
            &tool,
            "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n",
        );
        let preview = TestComparisonPreview::new(tool.clone(), request.clone()).unwrap();
        let command = adapter
            .comparison_command(&preview, baseline, candidate)
            .unwrap();
        let mut cancelled = TestResultJob::new().with_cancellation_timeout(Duration::from_secs(1));
        cancelled.start(command).await.unwrap();
        let _ = cancelled.next_event().await.unwrap();
        let _ = cancelled.next_event().await.unwrap();
        assert!(cancelled.cancel().await.unwrap());
        assert!(matches!(
            cancelled.next_event().await.unwrap(),
            TestResultRunnerEvent::Cancelled { forced: false, .. }
        ));

        executable(
            &tool,
            "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
        );
        let preview = TestComparisonPreview::new(tool.clone(), request.clone()).unwrap();
        let command = adapter
            .comparison_command(&preview, baseline, candidate)
            .unwrap();
        let mut timed_out = TestResultJob::new()
            .with_cancellation_timeout(Duration::from_millis(20))
            .with_operation_timeout(Duration::from_millis(20));
        timed_out.start(command.clone()).await.unwrap();
        let _ = timed_out.next_event().await.unwrap();
        loop {
            if matches!(
                timed_out.next_event().await.unwrap(),
                TestResultRunnerEvent::TimedOut { forced: true, .. }
            ) {
                break;
            }
        }

        let mut lost = TestResultJob::new();
        lost.start(command).await.unwrap();
        let _ = lost.next_event().await.unwrap();
        lost.lose_output_channel();
        assert!(matches!(
            lost.next_event().await.unwrap(),
            TestResultRunnerEvent::Lost { .. }
        ));
        assert!(!lost.cancel().await.unwrap());
        assert!(matches!(
            lost.next_event().await.unwrap(),
            TestResultRunnerEvent::CancellationRejected { .. }
        ));
    }
}
