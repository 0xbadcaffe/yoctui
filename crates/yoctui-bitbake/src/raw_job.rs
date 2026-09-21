include!("raw_job/command_planning.rs");

include!("raw_job/events_and_output.rs");

include!("raw_job/job_runner.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, fs};
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, RawExecutionPolicy,
        RawParameterValue, ToolIdentity, YoctoEnvironmentIdentity,
    };

    struct Fixture(PathBuf);

    impl Fixture {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-raw-job-{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn executable(&self, body: &str) -> PathBuf {
            let path = self.0.join("bitbake");
            crate::test_support::write_executable(&path, body);
            path
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fixture_authority(build: &Path, executable: &Path) -> DaemonCompatibilitySnapshot {
        let catalog = builtin_raw_catalog();
        let command = catalog
            .commands
            .iter()
            .find(|command| {
                matches!(
                    command.execution,
                    RawExecutionPolicy::Executable { ref template }
                        if template.interaction == RawInteractionMode::NoninteractiveJob
                )
            })
            .unwrap();
        let RawExecutionPolicy::Executable { template } = &command.execution else {
            unreachable!();
        };
        let required_capabilities = match &template.capabilities {
            yoctui_model::RawCapabilityRequirement::All { capabilities }
            | yoctui_model::RawCapabilityRequirement::Any { capabilities } => capabilities.clone(),
        };
        let capabilities = required_capabilities
            .iter()
            .copied()
            .map(|id| CapabilityRecord {
                id,
                state: CapabilityState::Available,
                evidence: vec![CapabilityEvidence {
                    kind: CapabilityEvidenceKind::DirectProbe,
                    outcome: CapabilityEvidenceOutcome::Positive,
                    subject: id.as_str().into(),
                    detail: "Raw job fixture".into(),
                    argv: vec!["bitbake".into(), "--help".into()],
                }],
            })
            .collect::<Vec<_>>();
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation: 7,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        build.into(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    available_tools: AuthoritativeValue::detected(
                        vec![ToolIdentity {
                            id: "bitbake".into(),
                            executable: executable.into(),
                            version: Some("2.18.0".into()),
                        }],
                        IdentityAuthority::ExecutableProbe,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities,
            },
            implementations: required_capabilities
                .into_iter()
                .map(|id| {
                    (
                        id,
                        CapabilityImplementation {
                            id: format!("{}.fixture", id.as_str()),
                            kind: CapabilityImplementationKind::Command,
                        },
                    )
                })
                .collect(),
        }
        .normalize()
        .unwrap()
    }

    fn request(authority: &DaemonCompatibilitySnapshot) -> RawConfirmedExecutionRequest {
        let catalog = builtin_raw_catalog();
        let command = catalog
            .commands
            .iter()
            .find(|command| {
                matches!(
                    command.execution,
                    RawExecutionPolicy::Executable { ref template }
                        if template.interaction == RawInteractionMode::NoninteractiveJob
                            && command.parameters.is_empty()
                )
            })
            .unwrap();
        let preview_request = RawPreviewRequest {
            catalog_version: catalog.version,
            command: command.id.clone(),
            parameters: BTreeMap::<_, RawParameterValue>::new(),
            additional_arguments: RawAdditionalArguments::from_vec(vec!["extra-target".into()])
                .unwrap(),
            capability_generation: authority.snapshot.generation,
            build_directory: authority
                .snapshot
                .environment
                .build_directory
                .value()
                .unwrap()
                .clone(),
        };
        let preview = catalog.preview(&preview_request, Some(authority)).unwrap();
        RawConfirmedExecutionRequest::from_reviewed_preview(
            RawRequestId::new("raw-request:fixture-1").unwrap(),
            catalog,
            &preview_request,
            &preview,
        )
        .unwrap()
    }

    fn spec(authority: &DaemonCompatibilitySnapshot) -> RawJobCommandSpec {
        RawJobPlanner::new(authority)
            .plan(
                &request(authority),
                RawJobId::new("raw-job:fixture-1").unwrap(),
                RawStreamId::new("raw-stream:fixture-stdout").unwrap(),
                RawStreamId::new("raw-stream:fixture-stderr").unwrap(),
            )
            .unwrap()
    }

    #[test]
    fn raw_job_planner_reconstructs_exact_native_argv_and_rejects_tampering_before_spawn() {
        let fixture = Fixture::new("planner");
        let executable = fixture.executable("#!/bin/sh\nexit 0\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let request = request(&authority);
        let command = RawJobPlanner::new(&authority)
            .plan(
                &request,
                RawJobId::new("raw-job:planner").unwrap(),
                RawStreamId::new("raw-stream:planner-out").unwrap(),
                RawStreamId::new("raw-stream:planner-err").unwrap(),
            )
            .unwrap();
        assert_eq!(command.executable(), executable);
        assert_eq!(command.current_directory(), fixture.0);
        assert_eq!(
            command.arguments().last().map(OsString::as_os_str),
            Some(std::ffi::OsStr::new("extra-target"))
        );
        assert!(!command.arguments().iter().any(|argument| argument == "sh"));

        let mut tampered = request;
        tampered.preview_digest.0[0] ^= 0xff;
        assert_eq!(
            RawJobPlanner::new(&authority).plan(
                &tampered,
                RawJobId::new("raw-job:tampered").unwrap(),
                RawStreamId::new("raw-stream:tampered-out").unwrap(),
                RawStreamId::new("raw-stream:tampered-err").unwrap(),
            ),
            Err(RawJobPlannerError::PreviewMismatch)
        );
    }

    #[tokio::test]
    async fn raw_job_runner_streams_bounded_unicode_and_reports_success_and_nonzero() {
        let fixture = Fixture::new("outcomes");
        let executable = fixture
            .executable("#!/bin/sh\nprintf 'hello 界\\n'\nprintf 'warning\\n' >&2\nexit 0\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let mut runner = RawJobRunner::new();
        runner.start(spec(&authority)).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            RawJobRunnerEvent::Started
        );
        let mut streams = Vec::new();
        loop {
            match runner.next_event().await.unwrap() {
                RawJobRunnerEvent::Output(chunk) => streams.push((chunk.stream, chunk.text)),
                RawJobRunnerEvent::Completed { exit_code } => {
                    assert_eq!(exit_code, 0);
                    break;
                }
                other => panic!("unexpected event: {other:?}"),
            }
        }
        assert!(streams.contains(&(RawOutputStream::Stdout, "hello 界".into())));
        assert!(streams.contains(&(RawOutputStream::Stderr, "warning".into())));

        let executable = fixture.executable("#!/bin/sh\nexit 9\n");
        let authority = fixture_authority(&fixture.0, &executable);
        runner.start(spec(&authority)).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            RawJobRunnerEvent::Started
        );
        assert!(matches!(
            runner.next_event().await.unwrap(),
            RawJobRunnerEvent::Failed {
                exit_code: Some(9),
                ..
            }
        ));
    }

    #[tokio::test]
    async fn raw_job_runner_cancels_gracefully_forcibly_times_out_and_reports_loss() {
        let fixture = Fixture::new("terminal");
        let executable = fixture
            .executable("#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let mut graceful = RawJobRunner::new().with_cancellation_timeout(Duration::from_secs(1));
        graceful.start(spec(&authority)).await.unwrap();
        graceful.next_event().await.unwrap();
        assert!(matches!(
            graceful.next_event().await.unwrap(),
            RawJobRunnerEvent::Output(_)
        ));
        assert!(graceful.cancel().await.unwrap());
        assert!(!graceful.cancel().await.unwrap());
        assert!(matches!(
            graceful.next_event().await.unwrap(),
            RawJobRunnerEvent::Cancelled { forced: false, .. }
        ));

        let executable =
            fixture.executable("#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do :; done\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let mut forced = RawJobRunner::new().with_cancellation_timeout(Duration::from_millis(20));
        forced.start(spec(&authority)).await.unwrap();
        forced.next_event().await.unwrap();
        assert!(matches!(
            forced.next_event().await.unwrap(),
            RawJobRunnerEvent::Output(_)
        ));
        assert!(forced.cancel().await.unwrap());
        assert!(matches!(
            forced.next_event().await.unwrap(),
            RawJobRunnerEvent::Cancelled { forced: true, .. }
        ));

        let executable = fixture.executable("#!/bin/sh\nwhile :; do sleep 1; done\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let mut timed = RawJobRunner::new()
            .with_operation_timeout(Duration::from_millis(20))
            .with_cancellation_timeout(Duration::from_millis(20));
        timed.start(spec(&authority)).await.unwrap();
        timed.next_event().await.unwrap();
        assert!(matches!(
            timed.next_event().await.unwrap(),
            RawJobRunnerEvent::TimedOut { .. }
        ));

        let executable = fixture.executable("#!/bin/sh\nwhile :; do sleep 1; done\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let mut lost = RawJobRunner::new();
        lost.start(spec(&authority)).await.unwrap();
        lost.next_event().await.unwrap();
        lost.lose_output_channel();
        assert!(matches!(
            lost.next_event().await.unwrap(),
            RawJobRunnerEvent::Lost { .. }
        ));
    }

    #[tokio::test]
    async fn raw_job_runner_truncates_oversized_lines_and_revalidates_at_start() {
        let fixture = Fixture::new("bounds");
        let executable = fixture.executable(&format!(
            "#!/bin/sh\nprintf '%s\\n' '{}'\n",
            "界".repeat(yoctui_model::MAX_RAW_OUTPUT_CHUNK_BYTES)
        ));
        let authority = fixture_authority(&fixture.0, &executable);
        let mut runner = RawJobRunner::new();
        runner.start(spec(&authority)).await.unwrap();
        runner.next_event().await.unwrap();
        let RawJobRunnerEvent::Output(chunk) = runner.next_event().await.unwrap() else {
            panic!("expected bounded output");
        };
        assert!(chunk.text.len() <= yoctui_model::MAX_RAW_OUTPUT_CHUNK_BYTES);
        assert!(chunk.truncated_bytes > 0);

        let command = spec(&authority);
        fs::remove_file(executable).unwrap();
        let mut denied = RawJobRunner::new();
        assert!(matches!(
            denied.start(command).await,
            Err(RawJobRunnerError::Authorization(_))
        ));
        assert!(!denied.is_active());

        let executable = fixture.executable("not an executable image\n");
        let authority = fixture_authority(&fixture.0, &executable);
        let mut rejected = RawJobRunner::new();
        assert!(matches!(
            rejected.start(spec(&authority)).await,
            Err(RawJobRunnerError::Spawn(_))
        ));
        assert!(!rejected.is_active());
    }
}
