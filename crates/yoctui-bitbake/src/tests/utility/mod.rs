use super::*;
use std::collections::BTreeMap;
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord, CapabilitySnapshot,
    CapabilityState, IdentityAuthority, ToolIdentity, YoctoEnvironmentIdentity,
};

fn authority(generation: u64, build: &Path, executable: &Path) -> DaemonCompatibilitySnapshot {
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    build.to_owned(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: AuthoritativeValue::detected(
                    vec![ToolIdentity {
                        id: "runqemu".into(),
                        executable: executable.to_owned(),
                        version: None,
                    }],
                    IdentityAuthority::ExecutableProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: vec![CapabilityRecord {
                id: CapabilityId::RunQemu,
                state: CapabilityState::Available,
                evidence: vec![CapabilityEvidence {
                    kind: CapabilityEvidenceKind::DirectProbe,
                    outcome: CapabilityEvidenceOutcome::Positive,
                    subject: "runqemu --help".into(),
                    detail: "The initialized runqemu executable accepted help.".into(),
                    argv: vec![executable.display().to_string(), "--help".into()],
                }],
            }],
        },
        implementations: BTreeMap::from([(
            CapabilityId::RunQemu,
            CapabilityImplementation {
                id: "runqemu.argv".into(),
                kind: CapabilityImplementationKind::Command,
            },
        )]),
    }
    .normalize()
    .unwrap()
}
mod utility_runner_parses_argv_and_rejects_shell_controls;
mod utility_runner_preview_is_indexed_and_cwd_bounded;

mod compatibility_utilities_authorizes_exact_snapshot_tool_and_implementation;

mod compatibility_utilities_rejects_stale_unknown_and_wrong_implementation;
