use super::*;
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityId, CapabilityRecord, CapabilityState, IdentityAuthority,
};

fn environment(build: &str, version: &str) -> YoctoEnvironmentIdentity {
    YoctoEnvironmentIdentity {
        build_directory: AuthoritativeValue::detected(
            build.into(),
            IdentityAuthority::InitializedEnvironment,
        ),
        bitbake_version: AuthoritativeValue::detected(
            version.into(),
            IdentityAuthority::BitBakeVersionProbe,
        ),
        ..YoctoEnvironmentIdentity::default()
    }
}

fn material<'a>(workspace: &'a str, daemon: &'a str) -> CapabilityFingerprintMaterial<'a> {
    CapabilityFingerprintMaterial {
        workspace_identity: workspace,
        initialized_environment: b"PATH=/poky/bitbake/bin\0BUILDDIR=/poky/build",
        layer_configuration: b"BBLAYERS=/poky/meta /poky/meta-poky",
        build_configuration: b"MACHINE=qemux86-64\nDISTRO=poky",
        daemon_workspace_identity: daemon,
    }
}

fn snapshot(generation: u64, environment: YoctoEnvironmentIdentity) -> CapabilitySnapshot {
    CapabilitySnapshot {
        generation,
        environment,
        capabilities: vec![CapabilityRecord {
            id: CapabilityId::BitBakeWorkspaceInspection,
            state: CapabilityState::Available,
            evidence: vec![CapabilityEvidence {
                kind: CapabilityEvidenceKind::BackendNegotiation,
                outcome: CapabilityEvidenceOutcome::Positive,
                subject: "workspace".into(),
                detail: "backend reports workspace inspection".into(),
                argv: Vec::new(),
            }],
        }],
    }
    .normalize()
    .unwrap()
}

mod compatibility_cache_reuses_only_exact_environment_key;

mod compatibility_cache_invalidates_each_project_environment_and_config_dimension;

mod compatibility_cache_rejects_stale_generation_environment_and_key;

mod compatibility_cache_fingerprint_is_deterministic_bounded_and_sensitive;

mod compatibility_cache_generation_overflow_fails_closed;
