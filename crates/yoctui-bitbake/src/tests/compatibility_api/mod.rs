use super::*;
use std::{collections::BTreeMap, path::PathBuf};
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord, CapabilitySnapshot,
    CapabilityState, IdentityAuthority, YoctoEnvironmentIdentity,
};

fn authority(
    generation: u64,
    version: &str,
    capabilities: &[(CapabilityId, &str)],
) -> DaemonCompatibilitySnapshot {
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    PathBuf::from("/work/build"),
                    IdentityAuthority::InitializedEnvironment,
                ),
                bitbake_version: AuthoritativeValue::detected(
                    version.into(),
                    IdentityAuthority::BitBakeVersionProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: capabilities
                .iter()
                .map(|(id, _)| CapabilityRecord {
                    id: *id,
                    state: CapabilityState::Available,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::BackendNegotiation,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: id.as_str().into(),
                        detail: "The initialized backend positively negotiated this behavior."
                            .into(),
                        argv: Vec::new(),
                    }],
                })
                .collect(),
        },
        implementations: capabilities
            .iter()
            .map(|(id, implementation)| {
                (
                    *id,
                    CapabilityImplementation {
                        id: (*implementation).into(),
                        kind: CapabilityImplementationKind::BackendApi,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
    }
    .normalize()
    .unwrap()
}

fn negotiate_all(authority: &mut BitBakeApiAuthority) {
    let payload = authority.bridge_handshake();
    authority
        .accept_negotiation(
            Some(payload.generation),
            &payload
                .capabilities
                .into_iter()
                .map(|capability| capability.id)
                .collect::<Vec<_>>(),
        )
        .unwrap();
}

mod compatibility_api_accepts_old_and_future_adapters_from_snapshot_not_version_policy;

mod compatibility_api_rejects_stale_environment_command_fallback_and_missing_negotiation;

mod compatibility_api_rejects_stale_or_unoffered_bridge_negotiation;
