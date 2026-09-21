use super::*;
use std::os::unix::fs::PermissionsExt;
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityId, CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
    CapabilitySnapshot, CapabilityState, IdentityAuthority, ToolIdentity, YoctoEnvironmentIdentity,
};

fn compatibility(
    build: &std::path::Path,
    executable: &std::path::Path,
) -> DaemonCompatibilitySnapshot {
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation: 1,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    build.to_owned(),
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
            capabilities: vec![CapabilityRecord {
                id: CapabilityId::DevtoolModify,
                state: CapabilityState::Available,
                evidence: vec![CapabilityEvidence {
                    kind: CapabilityEvidenceKind::DirectProbe,
                    outcome: CapabilityEvidenceOutcome::Positive,
                    subject: "devtool modify --help".into(),
                    detail: "Fixture exposes modify.".into(),
                    argv: vec![executable.display().to_string(), "--help".into()],
                }],
            }],
        },
        implementations: std::collections::BTreeMap::from([(
            CapabilityId::DevtoolModify,
            CapabilityImplementation {
                id: yoctui_bitbake::DEVTOOL_MODIFY_IMPLEMENTATION.into(),
                kind: CapabilityImplementationKind::Command,
            },
        )]),
    }
    .normalize()
    .unwrap()
}

mod compatibility_devtool_daemon_runner_uses_owned_snapshot_and_survives_client_scope;
