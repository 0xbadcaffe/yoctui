use super::*;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    sync::atomic::{AtomicU64, Ordering},
};
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityId, CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
    CapabilitySnapshot, CapabilityState, IdentityAuthority, YoctoEnvironmentIdentity,
};

fn operation_parts(operation: BitBakeCliOperation) -> (&'static str, &'static str) {
    match operation {
        BitBakeCliOperation::Status => (
            crate::compatibility_command::BITBAKE_SERVER_STATUS_ARGV_IMPLEMENTATION,
            "--status-only",
        ),
        BitBakeCliOperation::StartServer => (
            crate::compatibility_command::BITBAKE_SERVER_START_ARGV_IMPLEMENTATION,
            "--server-only",
        ),
        BitBakeCliOperation::StopServer => (
            crate::compatibility_command::BITBAKE_SERVER_STOP_ARGV_IMPLEMENTATION,
            "--kill-server",
        ),
    }
}

fn compatibility(
    build_dir: &Path,
    executable: &Path,
    operation: BitBakeCliOperation,
) -> DaemonCompatibilitySnapshot {
    let (implementation, _) = operation_parts(operation);
    let capability = match operation {
        BitBakeCliOperation::Status => CapabilityId::BitBakeServerStatus,
        BitBakeCliOperation::StartServer => CapabilityId::BitBakeServerStart,
        BitBakeCliOperation::StopServer => CapabilityId::BitBakeServerStop,
    };
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation: 1,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    build_dir.to_owned(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: AuthoritativeValue::detected(
                    vec![yoctui_model::ToolIdentity {
                        id: "bitbake".into(),
                        executable: executable.to_owned(),
                        version: None,
                    }],
                    IdentityAuthority::ExecutableProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: vec![CapabilityRecord {
                id: capability,
                state: CapabilityState::Available,
                evidence: vec![CapabilityEvidence {
                    kind: CapabilityEvidenceKind::DirectProbe,
                    outcome: CapabilityEvidenceOutcome::Positive,
                    subject: "BitBake server option".into(),
                    detail: "The exact server option was observed in help output.".into(),
                    argv: vec!["bitbake".into(), "--help".into()],
                }],
            }],
        },
        implementations: BTreeMap::from([(
            capability,
            CapabilityImplementation {
                id: implementation.into(),
                kind: CapabilityImplementationKind::Command,
            },
        )]),
    }
    .normalize()
    .unwrap()
}

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(unix)]
mod cli_control_retries_only_text_file_busy_spawn_failures;

fn fixture(body: &str) -> (PathBuf, PathBuf) {
    let nonce = std::time::SystemTime::UNIX_EPOCH
        .elapsed()
        .unwrap()
        .as_nanos();
    let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "yoctui-cli-control-{}-{nonce}-{sequence};no-shell",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let executable = root.join("fake bitbake;not-shell");
    fs::write(&executable, format!("#!/bin/sh\n{body}\n")).unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&executable, permissions).unwrap();
    (root, executable)
}

fn command(
    executable: PathBuf,
    build_dir: PathBuf,
    operation: BitBakeCliOperation,
    timeout: Duration,
    output_limit: usize,
) -> BitBakeCliCommand {
    let compatibility = compatibility(&build_dir, &executable, operation);
    BitBakeCliCommand::with_limits(
        executable,
        build_dir,
        BTreeMap::from([("MARKER".into(), "captured".into())]),
        &compatibility,
        compatibility.snapshot.generation,
        operation,
        timeout,
        output_limit,
    )
    .unwrap()
}

mod cli_control_previews_and_runs_exact_shell_free_server_operations;

mod cli_control_is_capability_aware_and_bounds_output_and_runtime;

mod cli_control_cancels_the_owned_process_group;
