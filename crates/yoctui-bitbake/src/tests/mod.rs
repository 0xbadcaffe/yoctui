//! Shared fixtures and regression modules.

use super::*;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn fixture_script(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("yoctui-{name}-{}-{nonce}", std::process::id()))
}

pub(crate) fn process_compatibility(
    build_dir: &std::path::Path,
    executable: &std::path::Path,
) -> yoctui_model::DaemonCompatibilitySnapshot {
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityId, CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, ToolIdentity,
        YoctoEnvironmentIdentity,
    };
    let capabilities = [
        (
            CapabilityId::BitBakeBuild,
            compatibility_command::BITBAKE_BUILD_ARGV_IMPLEMENTATION,
        ),
        (
            CapabilityId::BitBakeForceTask,
            compatibility_command::BITBAKE_FORCE_TASK_ARGV_IMPLEMENTATION,
        ),
        (
            CapabilityId::BitBakeGraphGeneration,
            compatibility_command::BITBAKE_GRAPH_ARGV_IMPLEMENTATION,
        ),
        (
            CapabilityId::BitBakeDumpSig,
            compatibility_command::BITBAKE_DUMPSIG_ARGV_IMPLEMENTATION,
        ),
        (
            CapabilityId::BitBakeDiffSigs,
            compatibility_command::BITBAKE_DIFFSIGS_ARGV_IMPLEMENTATION,
        ),
    ];
    yoctui_model::DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation: 1,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    build_dir.to_owned(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: AuthoritativeValue::detected(
                    vec![ToolIdentity {
                        id: "bitbake".into(),
                        executable: executable.to_owned(),
                        version: None,
                    }],
                    IdentityAuthority::ExecutableProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: capabilities
                .iter()
                .map(|(id, _)| CapabilityRecord {
                    id: *id,
                    state: CapabilityState::Available,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: format!("{} fixture probe", id.as_str()),
                        detail: "The fake BitBake command supports this exact test argv.".into(),
                        argv: vec!["bitbake".into(), "--help".into()],
                    }],
                })
                .collect(),
        },
        implementations: capabilities
            .into_iter()
            .map(|(id, implementation)| {
                (
                    id,
                    CapabilityImplementation {
                        id: implementation.into(),
                        kind: CapabilityImplementationKind::Command,
                    },
                )
            })
            .collect(),
    }
    .normalize()
    .unwrap()
}

pub(crate) fn shell_backend(script: PathBuf) -> ProcessBackend {
    let build_dir = std::env::temp_dir();
    ProcessBackend::with_command(
        build_dir.clone(),
        PathBuf::from("/bin/sh"),
        vec![script.into_os_string()],
    )
    .with_compatibility(process_compatibility(&build_dir, Path::new("/bin/sh")))
    .unwrap()
}

pub(crate) fn devtool_compatibility(
    build_dir: &Path,
    executable: &Path,
) -> yoctui_model::DaemonCompatibilitySnapshot {
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityId, CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, ToolIdentity,
        YoctoEnvironmentIdentity,
    };
    let capabilities = [
        (
            CapabilityId::DevtoolStatus,
            compatibility_devtool::DEVTOOL_STATUS_IMPLEMENTATION,
        ),
        (
            CapabilityId::DevtoolModify,
            compatibility_devtool::DEVTOOL_MODIFY_IMPLEMENTATION,
        ),
        (
            CapabilityId::DevtoolUpdateRecipe,
            compatibility_devtool::DEVTOOL_UPDATE_RECIPE_IMPLEMENTATION,
        ),
        (
            CapabilityId::DevtoolFinish,
            compatibility_devtool::DEVTOOL_FINISH_IMPLEMENTATION,
        ),
        (
            CapabilityId::DevtoolDeployTarget,
            compatibility_devtool::DEVTOOL_DEPLOY_TARGET_IMPLEMENTATION,
        ),
        (
            CapabilityId::DevtoolUndeployTarget,
            compatibility_devtool::DEVTOOL_UNDEPLOY_TARGET_IMPLEMENTATION,
        ),
        (
            CapabilityId::DevtoolReset,
            compatibility_devtool::DEVTOOL_RESET_IMPLEMENTATION,
        ),
        (
            CapabilityId::DevtoolUpgrade,
            compatibility_devtool::DEVTOOL_UPGRADE_IMPLEMENTATION,
        ),
    ];
    yoctui_model::DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation: 1,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    build_dir.to_owned(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: AuthoritativeValue::detected(
                    vec![ToolIdentity {
                        id: "devtool".into(),
                        executable: executable.to_owned(),
                        version: None,
                    }],
                    IdentityAuthority::ExecutableProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: capabilities
                .iter()
                .map(|(id, _)| CapabilityRecord {
                    id: *id,
                    state: CapabilityState::Available,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: format!("{} test probe", id.as_str()),
                        detail: "The fixture exposes this exact Devtool subcommand.".into(),
                        argv: vec![executable.display().to_string(), "--help".into()],
                    }],
                })
                .collect(),
        },
        implementations: capabilities
            .into_iter()
            .map(|(id, implementation)| {
                (
                    id,
                    CapabilityImplementation {
                        id: implementation.into(),
                        kind: CapabilityImplementationKind::Command,
                    },
                )
            })
            .collect(),
    }
    .normalize()
    .unwrap()
}

pub(crate) fn authorized_devtool_command(
    mut executable: PathBuf,
    operation: &DevtoolOperation,
) -> Result<DevtoolCommandSpec, DevtoolCompatibilityError> {
    if !executable.is_absolute() {
        executable = Path::new("/test/bin").join(executable);
    }
    let build_dir = std::env::temp_dir();
    let compatibility = devtool_compatibility(&build_dir, &executable);
    DevtoolCommandSpec::with_executable(
        executable,
        operation,
        &compatibility,
        compatibility.snapshot.generation,
        &build_dir,
    )
}

#[cfg(unix)]
pub(crate) fn fake_devtool_command(name: &str, body: &str) -> (PathBuf, DevtoolCommandSpec) {
    let script = fixture_script(name);
    fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).unwrap();
    let command = DevtoolCommandSpec {
        executable: PathBuf::from("/bin/sh"),
        arguments: vec![
            script.clone().into_os_string(),
            OsString::from("modify"),
            OsString::from("busybox"),
        ],
        capability_generation: 1,
        capability: yoctui_model::CapabilityId::DevtoolModify,
        build_directory: std::env::temp_dir(),
    };
    (script, command)
}
mod dependency_graph_process_backend_is_shell_free_and_rejects_failures;
mod process_backend_rejects_unavailable_variable_detail;
mod task_identity_statistics_decode_without_recipe_or_task_inference;

use super::bridge_backend::BridgeStderrTail;
use super::devtool_runner::{MAX_DEVTOOL_LINE_BYTES, parse_devtool_status, parse_git_status};
use super::process_output::MAX_PROCESS_LINE_BYTES;
