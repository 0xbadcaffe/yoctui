use std::{collections::BTreeSet, path::Path};

use thiserror::Error;
use yoctui_model::{CapabilityId, DaemonCompatibilitySnapshot};
use yoctui_protocol::{BridgeCapabilityData, BridgeCompatibilityData};

const VERSION_ADAPTER_PREFIX: &str = "tinfoil.adapter.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitBakeApiOperation {
    Workspace,
    Recipes,
    Layers,
    Variable,
    Dependencies,
    DependencyGraph,
    RecipeSources,
    RecipeMetadata,
    LayerRelationships,
    Build,
    ForceTask,
    Cancel,
    NativeEvents,
    ServerSocket,
}

impl BitBakeApiOperation {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Workspace => "workspace inspection",
            Self::Recipes => "recipe inventory",
            Self::Layers => "layer inventory",
            Self::Variable => "variable lookup",
            Self::Dependencies => "dependency inspection",
            Self::DependencyGraph => "dependency graph",
            Self::RecipeSources => "recipe source inspection",
            Self::RecipeMetadata => "recipe metadata inspection",
            Self::LayerRelationships => "layer relationship inspection",
            Self::Build => "build control and native events",
            Self::ForceTask => "forced task execution",
            Self::Cancel => "build cancellation",
            Self::NativeEvents => "native event stream",
            Self::ServerSocket => "server socket control",
        }
    }

    const fn requirements(self) -> &'static [(CapabilityId, &'static str)] {
        use CapabilityId as Id;
        match self {
            Self::Workspace => &[(Id::BitBakeWorkspaceInspection, "tinfoil.workspace")],
            Self::Recipes => &[(Id::BitBakeRecipeInventory, "tinfoil.recipes")],
            Self::Layers => &[(Id::BitBakeLayerInventory, "tinfoil.layers")],
            Self::Variable => &[(Id::BitBakeGetVar, "tinfoil.getvar")],
            Self::Dependencies => &[(Id::BitBakeRecipeDependencies, "tinfoil.dependencies")],
            Self::DependencyGraph => &[(Id::BitBakeDependencyGraph, "tinfoil.dependency_graph")],
            Self::RecipeSources => &[(Id::BitBakeRecipeSources, "tinfoil.recipe_sources")],
            Self::RecipeMetadata => &[(Id::BitBakeRecipeMetadata, "tinfoil.recipe_metadata")],
            Self::LayerRelationships => {
                &[(Id::BitBakeLayerRelationships, "tinfoil.layer_relationships")]
            }
            Self::Build => &[
                (Id::BitBakeBuild, "tinfoil.build"),
                (Id::BitBakeNativeEvents, "tinfoil.native_events"),
            ],
            Self::ForceTask => &[(Id::BitBakeForceTask, "tinfoil.force_task")],
            Self::Cancel => &[(Id::BitBakeCancellation, "tinfoil.cancel")],
            Self::NativeEvents => &[(Id::BitBakeNativeEvents, "tinfoil.native_events")],
            Self::ServerSocket => &[(Id::BitBakeServerSocket, "bitbake.server_socket")],
        }
    }
}

#[derive(Debug, Clone)]
pub struct BitBakeApiAuthority {
    compatibility: DaemonCompatibilitySnapshot,
    negotiated: BTreeSet<CapabilityId>,
}

impl BitBakeApiAuthority {
    pub fn new(
        compatibility: DaemonCompatibilitySnapshot,
        expected_generation: u64,
        build_directory: &Path,
    ) -> Result<Self, BitBakeApiCompatibilityError> {
        let compatibility = compatibility
            .normalize()
            .map_err(|error| BitBakeApiCompatibilityError::Invalid(error.to_string()))?;
        if compatibility.snapshot.generation != expected_generation {
            return Err(BitBakeApiCompatibilityError::StaleGeneration {
                expected: expected_generation,
                actual: compatibility.snapshot.generation,
            });
        }
        if compatibility
            .snapshot
            .environment
            .build_directory
            .value()
            .map(std::path::PathBuf::as_path)
            != Some(build_directory)
        {
            return Err(BitBakeApiCompatibilityError::EnvironmentMismatch);
        }
        let adapter_families = compatibility
            .implementations
            .values()
            .filter(|implementation| implementation.id.starts_with(VERSION_ADAPTER_PREFIX))
            .map(|implementation| implementation.id.as_str())
            .collect::<BTreeSet<_>>();
        if adapter_families.len() > 1 {
            return Err(BitBakeApiCompatibilityError::MixedAdapterFamilies);
        }
        Ok(Self {
            compatibility,
            negotiated: BTreeSet::new(),
        })
    }

    pub fn generation(&self) -> u64 {
        self.compatibility.snapshot.generation
    }

    pub fn compatibility_snapshot(&self) -> &DaemonCompatibilitySnapshot {
        &self.compatibility
    }

    pub fn bridge_handshake(&self) -> BridgeCompatibilityData {
        let capabilities = self
            .compatibility
            .snapshot
            .capabilities
            .iter()
            .filter(|record| record.state.is_enabled())
            .filter_map(|record| {
                let implementation = self.compatibility.implementations.get(&record.id)?;
                is_api_implementation(record.id, &implementation.id).then(|| BridgeCapabilityData {
                    id: record.id.as_str().into(),
                    implementation: implementation.id.clone(),
                })
            })
            .collect();
        BridgeCompatibilityData {
            generation: self.generation(),
            build_directory: self
                .compatibility
                .snapshot
                .environment
                .build_directory
                .value()
                .expect("constructor requires a detected build directory")
                .display()
                .to_string(),
            capabilities,
        }
    }

    pub fn accept_negotiation(
        &mut self,
        generation: Option<u64>,
        capabilities: &[String],
    ) -> Result<(), BitBakeApiCompatibilityError> {
        if generation != Some(self.generation()) {
            return Err(BitBakeApiCompatibilityError::NegotiationGeneration {
                expected: self.generation(),
                actual: generation,
            });
        }
        if capabilities.len() > CapabilityId::ALL.len() {
            return Err(BitBakeApiCompatibilityError::NegotiationOversized);
        }
        let offered = self
            .bridge_handshake()
            .capabilities
            .into_iter()
            .map(|capability| capability.id)
            .collect::<BTreeSet<_>>();
        let mut negotiated = BTreeSet::new();
        for id in capabilities {
            if !offered.contains(id) {
                return Err(BitBakeApiCompatibilityError::UnexpectedNegotiated(
                    id.clone(),
                ));
            }
            let parsed = CapabilityId::ALL
                .iter()
                .copied()
                .find(|candidate| candidate.as_str() == id)
                .ok_or_else(|| BitBakeApiCompatibilityError::UnexpectedNegotiated(id.clone()))?;
            if !negotiated.insert(parsed) {
                return Err(BitBakeApiCompatibilityError::DuplicateNegotiated(
                    id.clone(),
                ));
            }
        }
        self.negotiated = negotiated;
        Ok(())
    }

    pub fn require(
        &self,
        operation: BitBakeApiOperation,
    ) -> Result<(), BitBakeApiCompatibilityError> {
        for (id, direct_implementation) in operation.requirements() {
            let record = self
                .compatibility
                .snapshot
                .capability(*id)
                .ok_or(BitBakeApiCompatibilityError::CapabilityMissing { capability: *id })?;
            if !record.state.is_enabled() {
                return Err(BitBakeApiCompatibilityError::Unavailable {
                    capability: *id,
                    reason: record
                        .state
                        .reason()
                        .map(|reason| reason.message.clone())
                        .unwrap_or_else(|| "No positive capability evidence is available.".into()),
                });
            }
            let implementation =
                self.compatibility.implementations.get(id).ok_or(
                    BitBakeApiCompatibilityError::ImplementationMissing { capability: *id },
                )?;
            if implementation.id != *direct_implementation
                && !implementation.id.starts_with(VERSION_ADAPTER_PREFIX)
            {
                return Err(BitBakeApiCompatibilityError::ImplementationMismatch {
                    capability: *id,
                    selected: implementation.id.clone(),
                    required: (*direct_implementation).into(),
                });
            }
            if !self.negotiated.contains(id) {
                return Err(BitBakeApiCompatibilityError::NotNegotiated {
                    operation: operation.name(),
                    capability: *id,
                });
            }
        }
        Ok(())
    }
}

fn is_api_implementation(id: CapabilityId, implementation: &str) -> bool {
    implementation.starts_with(VERSION_ADAPTER_PREFIX)
        || BitBakeApiOperation::Workspace
            .requirements()
            .iter()
            .chain(BitBakeApiOperation::Recipes.requirements())
            .chain(BitBakeApiOperation::Layers.requirements())
            .chain(BitBakeApiOperation::Variable.requirements())
            .chain(BitBakeApiOperation::Dependencies.requirements())
            .chain(BitBakeApiOperation::DependencyGraph.requirements())
            .chain(BitBakeApiOperation::RecipeSources.requirements())
            .chain(BitBakeApiOperation::RecipeMetadata.requirements())
            .chain(BitBakeApiOperation::LayerRelationships.requirements())
            .chain(BitBakeApiOperation::Build.requirements())
            .chain(BitBakeApiOperation::ForceTask.requirements())
            .chain(BitBakeApiOperation::Cancel.requirements())
            .chain(BitBakeApiOperation::NativeEvents.requirements())
            .chain(BitBakeApiOperation::ServerSocket.requirements())
            .any(|(candidate, direct)| *candidate == id && *direct == implementation)
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BitBakeApiCompatibilityError {
    #[error("invalid compatibility snapshot: {0}")]
    Invalid(String),
    #[error("stale compatibility generation: expected {expected}, got {actual}")]
    StaleGeneration { expected: u64, actual: u64 },
    #[error("compatibility snapshot belongs to another build environment")]
    EnvironmentMismatch,
    #[error("compatibility snapshot selects conflicting Tinfoil adapter families")]
    MixedAdapterFamilies,
    #[error("capability {capability:?} is absent from the compatibility snapshot")]
    CapabilityMissing { capability: CapabilityId },
    #[error("capability {capability:?} is unavailable: {reason}")]
    Unavailable {
        capability: CapabilityId,
        reason: String,
    },
    #[error("capability {capability:?} has no selected implementation")]
    ImplementationMissing { capability: CapabilityId },
    #[error(
        "capability {capability:?} selected implementation {selected}, not required API implementation {required}"
    )]
    ImplementationMismatch {
        capability: CapabilityId,
        selected: String,
        required: String,
    },
    #[error("bridge compatibility generation mismatch: expected {expected}, got {actual:?}")]
    NegotiationGeneration { expected: u64, actual: Option<u64> },
    #[error("bridge negotiated capability not offered by the daemon snapshot: {0}")]
    UnexpectedNegotiated(String),
    #[error("bridge repeated negotiated capability: {0}")]
    DuplicateNegotiated(String),
    #[error("bridge compatibility negotiation is oversized")]
    NegotiationOversized,
    #[error("{operation} is unavailable because the bridge did not negotiate {capability:?}")]
    NotNegotiated {
        operation: &'static str,
        capability: CapabilityId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, path::PathBuf};
    use yoctui_model::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityImplementation, CapabilityImplementationKind, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, IdentityAuthority, YoctoEnvironmentIdentity,
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

    #[test]
    fn compatibility_api_accepts_old_and_future_adapters_from_snapshot_not_version_policy() {
        let requirements = [
            (
                CapabilityId::BitBakeWorkspaceInspection,
                "tinfoil.adapter.legacy",
            ),
            (CapabilityId::BitBakeBuild, "tinfoil.adapter.legacy"),
            (CapabilityId::BitBakeNativeEvents, "tinfoil.adapter.legacy"),
        ];
        let mut old = BitBakeApiAuthority::new(
            authority(4, "1.52", &requirements),
            4,
            Path::new("/work/build"),
        )
        .unwrap();
        negotiate_all(&mut old);
        old.require(BitBakeApiOperation::Workspace).unwrap();
        old.require(BitBakeApiOperation::Build).unwrap();

        let future_requirements = [
            (
                CapabilityId::BitBakeWorkspaceInspection,
                "tinfoil.workspace",
            ),
            (CapabilityId::BitBakeNativeEvents, "tinfoil.native_events"),
        ];
        let mut future = BitBakeApiAuthority::new(
            authority(5, "99.0", &future_requirements),
            5,
            Path::new("/work/build"),
        )
        .unwrap();
        negotiate_all(&mut future);
        future.require(BitBakeApiOperation::Workspace).unwrap();
        future.require(BitBakeApiOperation::NativeEvents).unwrap();
    }

    #[test]
    fn compatibility_api_rejects_stale_environment_command_fallback_and_missing_negotiation() {
        let snapshot = authority(
            8,
            "2.18",
            &[(CapabilityId::BitBakeGetVar, "bitbake.environment_lookup")],
        );
        assert!(matches!(
            BitBakeApiAuthority::new(snapshot.clone(), 7, Path::new("/work/build")),
            Err(BitBakeApiCompatibilityError::StaleGeneration { .. })
        ));
        assert!(matches!(
            BitBakeApiAuthority::new(snapshot.clone(), 8, Path::new("/other/build")),
            Err(BitBakeApiCompatibilityError::EnvironmentMismatch)
        ));
        let mut api = BitBakeApiAuthority::new(snapshot, 8, Path::new("/work/build")).unwrap();
        api.accept_negotiation(Some(8), &[]).unwrap();
        assert!(matches!(
            api.require(BitBakeApiOperation::Variable),
            Err(BitBakeApiCompatibilityError::ImplementationMismatch { .. })
        ));

        let direct = authority(
            9,
            "2.18",
            &[(CapabilityId::BitBakeGetVar, "tinfoil.getvar")],
        );
        let mut api = BitBakeApiAuthority::new(direct, 9, Path::new("/work/build")).unwrap();
        api.accept_negotiation(Some(9), &[]).unwrap();
        assert!(matches!(
            api.require(BitBakeApiOperation::Variable),
            Err(BitBakeApiCompatibilityError::NotNegotiated { .. })
        ));
    }

    #[test]
    fn compatibility_api_rejects_stale_or_unoffered_bridge_negotiation() {
        let snapshot = authority(
            11,
            "2.18",
            &[(
                CapabilityId::BitBakeWorkspaceInspection,
                "tinfoil.workspace",
            )],
        );
        let mut api = BitBakeApiAuthority::new(snapshot, 11, Path::new("/work/build")).unwrap();
        assert!(matches!(
            api.accept_negotiation(Some(10), &[]),
            Err(BitBakeApiCompatibilityError::NegotiationGeneration { .. })
        ));
        assert!(matches!(
            api.accept_negotiation(Some(11), &["bitbake.build".into()]),
            Err(BitBakeApiCompatibilityError::UnexpectedNegotiated(_))
        ));
    }
}
