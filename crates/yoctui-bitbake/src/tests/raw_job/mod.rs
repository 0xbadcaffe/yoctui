use super::*;
use std::{collections::BTreeMap, fs};
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord, CapabilitySnapshot,
    CapabilityState, IdentityAuthority, RawExecutionPolicy, RawParameterValue, ToolIdentity,
    YoctoEnvironmentIdentity,
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

mod raw_job_planner_reconstructs_exact_native_argv_and_rejects_tampering_before_spawn;

mod raw_job_runner_streams_bounded_unicode_and_reports_success_and_nonzero;

mod raw_job_runner_cancels_gracefully_forcibly_times_out_and_reports_loss;

mod raw_job_runner_truncates_oversized_lines_and_revalidates_at_start;
