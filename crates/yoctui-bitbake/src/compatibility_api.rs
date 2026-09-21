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
#[path = "tests/compatibility_api/mod.rs"]
mod tests;
