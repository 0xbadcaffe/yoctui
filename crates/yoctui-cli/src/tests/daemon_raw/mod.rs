use super::*;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt, path::PathBuf};
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord, CapabilitySnapshot,
    CapabilityState, IdentityAuthority, RawAdditionalArguments, RawAttachmentState,
    RawCapabilityRequirement, RawExecutionOutcome, RawExecutionPolicy, RawInteractionMode,
    RawParameterValue, RawPreviewRequest, ToolIdentity, YoctoEnvironmentIdentity,
    builtin_raw_catalog,
};
use yoctui_protocol::daemon::{DaemonSnapshot, RawExecutionRequestData};

struct Fixture(PathBuf);

impl Fixture {
    fn new(name: &str, body: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "yoctui-daemon-raw-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        let executable = path.join("bitbake");
        fs::write(&executable, body).unwrap();
        let mut permissions = fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).unwrap();
        Self(path)
    }

    fn executable(&self) -> PathBuf {
        self.0.join("bitbake")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn authority_for(
    fixture: &Fixture,
    interaction: RawInteractionMode,
) -> DaemonCompatibilitySnapshot {
    let catalog = builtin_raw_catalog();
    let command = catalog
        .commands
        .iter()
        .find(|command| {
            matches!(
                command.execution,
                RawExecutionPolicy::Executable { ref template }
                    if template.interaction == interaction
                        && (interaction == RawInteractionMode::InteractivePty
                            || command.parameters.is_empty())
            )
        })
        .unwrap();
    let RawExecutionPolicy::Executable { template } = &command.execution else {
        unreachable!();
    };
    let required = match &template.capabilities {
        RawCapabilityRequirement::All { capabilities }
        | RawCapabilityRequirement::Any { capabilities } => capabilities.clone(),
    };
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation: 17,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    fixture.0.clone(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: AuthoritativeValue::detected(
                    vec![ToolIdentity {
                        id: "bitbake".into(),
                        executable: fixture.executable(),
                        version: Some("fixture".into()),
                    }],
                    IdentityAuthority::ExecutableProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: required
                .iter()
                .copied()
                .map(|id| CapabilityRecord {
                    id,
                    state: CapabilityState::Available,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: id.as_str().into(),
                        detail: "Raw daemon fixture".into(),
                        argv: vec!["bitbake".into(), "--help".into()],
                    }],
                })
                .collect(),
        },
        implementations: required
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

fn authority(fixture: &Fixture) -> DaemonCompatibilitySnapshot {
    authority_for(fixture, RawInteractionMode::NoninteractiveJob)
}

fn request(authority: &DaemonCompatibilitySnapshot, id: &str) -> RawExecutionRequestData {
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
        additional_arguments: RawAdditionalArguments::default(),
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
    let confirmed = yoctui_model::RawConfirmedExecutionRequest::from_reviewed_preview(
        RawRequestId::new(id).unwrap(),
        catalog,
        &preview_request,
        &preview,
    )
    .unwrap();
    yoctui_app::raw_execution_request_to_protocol(&confirmed).unwrap()
}

fn pty_request(authority: &DaemonCompatibilitySnapshot, id: &str) -> RawExecutionRequestData {
    let catalog = builtin_raw_catalog();
    let command = catalog
        .commands
        .iter()
        .find(|command| {
            matches!(
                command.execution,
                RawExecutionPolicy::Executable { ref template }
                    if template.interaction == RawInteractionMode::InteractivePty
                        && command.parameters.len() == 1
            )
        })
        .unwrap();
    let parameter = command.parameters.first().unwrap();
    let parameters = BTreeMap::from([(
        parameter.id.clone(),
        RawParameterValue::Target("core-image-minimal".into()),
    )]);
    let preview_request = RawPreviewRequest {
        catalog_version: catalog.version,
        command: command.id.clone(),
        parameters,
        additional_arguments: RawAdditionalArguments::default(),
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
    let confirmed = yoctui_model::RawConfirmedExecutionRequest::from_reviewed_preview(
        RawRequestId::new(id).unwrap(),
        catalog,
        &preview_request,
        &preview,
    )
    .unwrap();
    yoctui_app::raw_execution_request_to_protocol(&confirmed).unwrap()
}

async fn next_event(supervisor: &mut DaemonRawSupervisor) -> DaemonRawEvent {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(event) = supervisor.try_event() {
                return event;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
}

async fn next_pty_event(
    supervisor: &mut crate::daemon_pty::DaemonPtySupervisor,
) -> crate::daemon_pty::DaemonPtyEvent {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(event) = supervisor.try_event() {
                return event;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
}

mod raw_job_supervisor_journals_single_graceful_cancellation;
mod raw_job_supervisor_survives_detach_replays_output_and_rejects_duplicate;
mod raw_output_job_attachment_is_ordered_idempotent_and_does_not_cancel;
mod raw_pty_explicit_termination_is_required_and_reports_cancelled;
mod raw_pty_rejects_cross_route_stale_tampered_duplicate_and_tracks_cancel_and_loss;
mod raw_pty_uses_authorized_native_argv_and_survives_detach_resize_and_input;
