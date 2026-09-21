use std::collections::BTreeMap;

use yoctui_model::{
    AuthoritativeValue, BackendIdentity, CapabilityCatalog, CapabilityEvidence,
    CapabilityEvidenceKind, CapabilityEvidenceOutcome, CapabilityId, CapabilityImplementation,
    CapabilityImplementationKind, CapabilityReason, CapabilityState, DaemonCompatibilitySnapshot,
    DistroIdentity, IdentityAuthority, LayerSeriesIdentity, ProtocolIdentity, ReleaseIdentity,
    SourceRootIdentity, SourceRootKind, ToolIdentity, YoctoEnvironmentIdentity,
};

use crate::{
    CapabilityProbeObservation, CapabilityProbeStatus, CapabilityResolver,
    ResolvedCapabilitySnapshot,
};

/// Policy role only. These labels never claim that the represented release is supported or live.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompatibilityFixtureRole {
    OldestPolicyCandidate,
    IntermediateRepresentative,
    CurrentStableCandidate,
    LatestSupportCandidate,
    FutureUnknown,
}

impl CompatibilityFixtureRole {
    pub const ALL: [Self; 5] = [
        Self::OldestPolicyCandidate,
        Self::IntermediateRepresentative,
        Self::CurrentStableCandidate,
        Self::LatestSupportCandidate,
        Self::FutureUnknown,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OldestPolicyCandidate => "oldest-policy-candidate",
            Self::IntermediateRepresentative => "intermediate-representative",
            Self::CurrentStableCandidate => "current-stable-candidate",
            Self::LatestSupportCandidate => "latest-support-candidate",
            Self::FutureUnknown => "future-unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureCapabilityState {
    Available,
    Limited,
    Unavailable,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureCapabilityExpectation {
    pub id: CapabilityId,
    pub state: FixtureCapabilityState,
    pub implementation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReleaseCapabilityFixture {
    pub role: CompatibilityFixtureRole,
    pub fixture_only: bool,
    pub evidence_level: &'static str,
    pub environment: YoctoEnvironmentIdentity,
    pub observations: BTreeMap<CapabilityId, Vec<CapabilityProbeObservation>>,
    pub expectations: Vec<FixtureCapabilityExpectation>,
}

impl ReleaseCapabilityFixture {
    pub fn resolve(&self, generation: u64) -> ResolvedCapabilitySnapshot {
        CapabilityResolver::default()
            .resolve_snapshot(
                generation,
                self.environment.clone(),
                &CapabilityCatalog::builtin(),
                &self.observations,
            )
            .expect("static compatibility fixture must normalize")
    }

    /// Returns one command-focused authority shared by every command-planner test.
    ///
    /// The command profile is explicit direct fixture evidence. It is not a support claim and it
    /// does not infer commands from the fixture's release number.
    pub fn command_authority(&self, generation: u64) -> DaemonCompatibilitySnapshot {
        let mut resolved = self.resolve(generation);
        let modern = self.role != CompatibilityFixtureRole::OldestPolicyCandidate;
        let mut set = |id: CapabilityId, implementation: Option<&str>| {
            let record = resolved
                .snapshot
                .capabilities
                .iter_mut()
                .find(|record| record.id == id)
                .expect("every command capability must be cataloged");
            let subject = format!("{} {}", self.role.as_str(), id.as_str());
            match implementation {
                Some(implementation) => {
                    record.state = CapabilityState::Available;
                    record.evidence = vec![fixture_command_evidence(
                        CapabilityEvidenceOutcome::Positive,
                        &subject,
                    )];
                    resolved.implementations.insert(
                        id,
                        CapabilityImplementation {
                            id: implementation.into(),
                            kind: CapabilityImplementationKind::Command,
                        },
                    );
                }
                None => {
                    record.state = CapabilityState::Unavailable {
                        reason: CapabilityReason::new(
                            "fixture.command_absent",
                            format!(
                                "The {} fixture does not expose {}.",
                                self.role.as_str(),
                                id.as_str()
                            ),
                            Some(format!("Required capability: {}", id.as_str())),
                        )
                        .expect("static fixture reason must normalize"),
                    };
                    record.evidence = vec![fixture_command_evidence(
                        CapabilityEvidenceOutcome::Negative,
                        &subject,
                    )];
                    resolved.implementations.remove(&id);
                }
            }
        };

        for (id, implementation) in [
            (CapabilityId::BitBakeBuild, "bitbake.build.argv"),
            (CapabilityId::BitBakeForceTask, "bitbake.force_task.argv"),
            (
                CapabilityId::BitBakeEnvironmentDump,
                "bitbake.environment_dump.argv",
            ),
            (CapabilityId::BitBakeGraphGeneration, "bitbake.graph.argv"),
            (CapabilityId::BitBakeDumpSig, "bitbake_dumpsig.argv"),
            (CapabilityId::BitBakeDiffSigs, "bitbake_diffsigs.argv"),
            (CapabilityId::DevtoolModify, "devtool.modify.argv"),
            (CapabilityId::RecipetoolCreate, "recipetool.create.argv"),
            (
                CapabilityId::RecipetoolAppendFile,
                "recipetool.appendfile.argv",
            ),
            (
                CapabilityId::BitBakeLayersShowLayers,
                "bitbake_layers.show_layers.argv",
            ),
            (
                CapabilityId::BitBakeLayersAddLayer,
                "bitbake_layers.add_layer.argv",
            ),
            (
                CapabilityId::PkgDataListPackages,
                "pkgdata.list_packages.argv",
            ),
            (
                CapabilityId::PkgDataPackageInfo,
                "pkgdata.package_info.argv",
            ),
            (
                CapabilityId::PkgDataListPackageFiles,
                "pkgdata.list_package_files.argv",
            ),
            (CapabilityId::PkgDataReadValue, "pkgdata.read_value.argv"),
        ] {
            set(id, Some(implementation));
        }
        set(
            CapabilityId::BitBakeGetVar,
            Some(if modern {
                "bitbake_getvar.argv"
            } else {
                "bitbake.environment_lookup"
            }),
        );
        for (id, implementation) in [
            (CapabilityId::DevtoolUpgrade, "devtool.upgrade.argv"),
            (
                CapabilityId::RecipetoolCreateOutfile,
                "recipetool.create.outfile.argv",
            ),
            (
                CapabilityId::BitBakeLayersCreateAndAddLayer,
                "bitbake_layers.create_and_add_layer.argv",
            ),
        ] {
            set(id, modern.then_some(implementation));
        }

        DaemonCompatibilitySnapshot {
            snapshot: resolved.snapshot,
            implementations: resolved.implementations,
        }
        .normalize()
        .expect("static command fixture authority must normalize")
    }
}
