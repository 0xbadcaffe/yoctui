use crate::{
    App, BuildRequest, CapabilityId, CapabilityState, DaemonCompatibilitySnapshot, Dialog, Effect,
    MaintenanceDialog, MaintenanceEffect, MaintenanceOperation, QaDialog, QaEffect, Screen,
    SdkOperation, SecurityDialog, SecurityEffect, SecurityOperation, TestFamily, TestLaunchPreview,
    TestOperation, WicOperation,
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkspaceDestination {
    Dashboard,
    Recipes,
    Layers,
    Configuration,
    Tasks,
    BuildHistory,
    Logs,
    Errors,
    Dependencies,
    Signatures,
    Packages,
    Images,
    Sdk,
    Testing,
    Security,
    Qa,
    Devtool,
    QemuWic,
    Maintenance,
    ProjectProfiles,
    TerminalSessions,
    BuildEnvironment,
    Compatibility,
    Settings,
    Help,
}

impl WorkspaceDestination {
    pub const ALL: [Self; 25] = [
        Self::Dashboard,
        Self::Recipes,
        Self::Layers,
        Self::Configuration,
        Self::Tasks,
        Self::BuildHistory,
        Self::Logs,
        Self::Errors,
        Self::Dependencies,
        Self::Signatures,
        Self::Packages,
        Self::Images,
        Self::Sdk,
        Self::Testing,
        Self::Security,
        Self::Qa,
        Self::Devtool,
        Self::QemuWic,
        Self::Maintenance,
        Self::ProjectProfiles,
        Self::TerminalSessions,
        Self::BuildEnvironment,
        Self::Compatibility,
        Self::Settings,
        Self::Help,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceEffectRequirement {
    ClientLocal,
    /// Safe environment inspection belongs to the daemon probe coordinator;
    /// a client must not execute it to infer support independently.
    DaemonProbe {
        capabilities: Vec<CapabilityId>,
    },
    Capabilities {
        /// Every ID in this set is required.
        all: Vec<CapabilityId>,
        /// At least one ID in this set is required when non-empty.
        any: Vec<CapabilityId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceAvailabilityState {
    Available,
    AvailableWithLimitations,
    Unavailable,
    Unsupported,
    Unknown,
}

impl WorkspaceAvailabilityState {
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Available | Self::AvailableWithLimitations)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCapabilityIssue {
    pub capability: Option<CapabilityId>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceAvailability {
    pub state: WorkspaceAvailabilityState,
    pub issues: Vec<WorkspaceCapabilityIssue>,
    pub implementations: Vec<(CapabilityId, String)>,
}

impl WorkspaceAvailability {
    pub const fn is_enabled(&self) -> bool {
        self.state.is_enabled()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceCompatibilityState {
    authority: Option<DaemonCompatibilitySnapshot>,
}

impl WorkspaceCompatibilityState {
    pub const fn authority(&self) -> Option<&DaemonCompatibilitySnapshot> {
        self.authority.as_ref()
    }

    pub fn availability(&self, requirement: &WorkspaceEffectRequirement) -> WorkspaceAvailability {
        workspace_requirement_availability(self.authority(), requirement)
    }

    pub fn install(
        &mut self,
        authority: DaemonCompatibilitySnapshot,
    ) -> Result<WorkspaceSnapshotInstall, WorkspaceCompatibilityError> {
        let authority = authority
            .normalize()
            .map_err(|error| WorkspaceCompatibilityError::InvalidSnapshot(error.to_string()))?;
        if let Some(current) = self.authority.as_ref() {
            if authority.snapshot.generation < current.snapshot.generation {
                return Err(WorkspaceCompatibilityError::StaleGeneration {
                    current: current.snapshot.generation,
                    received: authority.snapshot.generation,
                });
            }
            if authority.snapshot.generation == current.snapshot.generation {
                if &authority == current {
                    return Ok(WorkspaceSnapshotInstall::Unchanged);
                }
                return Err(WorkspaceCompatibilityError::ConflictingGeneration(
                    authority.snapshot.generation,
                ));
            }
        }
        self.authority = Some(authority);
        Ok(WorkspaceSnapshotInstall::Replaced)
    }

    pub fn invalidate(&mut self) {
        self.authority = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSnapshotInstall {
    Replaced,
    Unchanged,
    Invalidated,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorkspaceCompatibilityError {
    #[error("invalid workspace capability snapshot: {0}")]
    InvalidSnapshot(String),
    #[error("stale workspace capability snapshot: current {current}, received {received}")]
    StaleGeneration { current: u64, received: u64 },
    #[error("workspace capability generation {0} conflicts with installed authority")]
    ConflictingGeneration(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRevalidation {
    pub install: WorkspaceSnapshotInstall,
    pub closed_dialog: bool,
    pub reason: Option<String>,
}

impl WorkspaceEffectRequirement {
    pub(crate) fn one(id: CapabilityId) -> Self {
        Self::Capabilities {
            all: vec![id],
            any: Vec::new(),
        }
    }

    pub(crate) fn all(ids: &[CapabilityId]) -> Self {
        Self::Capabilities {
            all: ids.to_vec(),
            any: Vec::new(),
        }
    }

    pub(crate) fn all_and_any(all: &[CapabilityId], any: &[CapabilityId]) -> Self {
        Self::Capabilities {
            all: all.to_vec(),
            any: any.to_vec(),
        }
    }

    fn probe(ids: &[CapabilityId]) -> Self {
        Self::DaemonProbe {
            capabilities: ids.to_vec(),
        }
    }
}

pub fn workspace_requirement_availability(
    authority: Option<&DaemonCompatibilitySnapshot>,
    requirement: &WorkspaceEffectRequirement,
) -> WorkspaceAvailability {
    match requirement {
        WorkspaceEffectRequirement::ClientLocal => WorkspaceAvailability {
            state: WorkspaceAvailabilityState::Available,
            issues: Vec::new(),
            implementations: Vec::new(),
        },
        WorkspaceEffectRequirement::DaemonProbe { capabilities } => WorkspaceAvailability {
            state: WorkspaceAvailabilityState::Unsupported,
            issues: vec![WorkspaceCapabilityIssue {
                capability: None,
                reason: format!(
                    "Environment probing is daemon-owned; request a correlated reprobe for: {}.",
                    capability_names(capabilities)
                ),
            }],
            implementations: Vec::new(),
        },
        WorkspaceEffectRequirement::Capabilities { all, any } => {
            capability_requirement_availability(authority, all, any)
        }
    }
}

fn capability_requirement_availability(
    authority: Option<&DaemonCompatibilitySnapshot>,
    all: &[CapabilityId],
    any: &[CapabilityId],
) -> WorkspaceAvailability {
    let Some(authority) = authority else {
        let capabilities = all.iter().chain(any).copied().collect::<Vec<_>>();
        return WorkspaceAvailability {
            state: WorkspaceAvailabilityState::Unknown,
            issues: capabilities
                .into_iter()
                .map(|capability| WorkspaceCapabilityIssue {
                    capability: Some(capability),
                    reason: format!(
                        "No current environment capability snapshot: {}.",
                        capability.as_str()
                    ),
                })
                .collect(),
            implementations: Vec::new(),
        };
    };

    let all_results = all
        .iter()
        .map(|id| capability_result(authority, *id))
        .collect::<Vec<_>>();
    let any_results = any
        .iter()
        .map(|id| capability_result(authority, *id))
        .collect::<Vec<_>>();
    let all_satisfied = all_results.iter().all(CapabilityResult::is_enabled);
    let selected_any = any_results.iter().find(|result| result.is_enabled());
    let any_satisfied = any.is_empty() || selected_any.is_some();

    if all_satisfied && any_satisfied {
        let selected = all_results.iter().chain(selected_any).collect::<Vec<_>>();
        let limited = selected.iter().any(|result| result.limited);
        let issues = selected
            .iter()
            .filter_map(|result| {
                result
                    .reason
                    .as_ref()
                    .map(|reason| WorkspaceCapabilityIssue {
                        capability: Some(result.id),
                        reason: reason.clone(),
                    })
            })
            .collect();
        let implementations = selected
            .iter()
            .filter_map(|result| {
                result
                    .implementation
                    .as_ref()
                    .map(|implementation| (result.id, implementation.clone()))
            })
            .collect();
        return WorkspaceAvailability {
            state: if limited {
                WorkspaceAvailabilityState::AvailableWithLimitations
            } else {
                WorkspaceAvailabilityState::Available
            },
            issues,
            implementations,
        };
    }

    let failures = all_results
        .into_iter()
        .filter(|result| !result.is_enabled())
        .chain(
            (!any_satisfied)
                .then_some(any_results)
                .into_iter()
                .flatten(),
        )
        .collect::<Vec<_>>();
    let state = if failures
        .iter()
        .any(|result| result.state == WorkspaceAvailabilityState::Unknown)
    {
        WorkspaceAvailabilityState::Unknown
    } else if !failures.is_empty()
        && failures
            .iter()
            .all(|result| result.state == WorkspaceAvailabilityState::Unsupported)
    {
        WorkspaceAvailabilityState::Unsupported
    } else {
        WorkspaceAvailabilityState::Unavailable
    };
    WorkspaceAvailability {
        state,
        issues: failures
            .into_iter()
            .map(|result| WorkspaceCapabilityIssue {
                capability: Some(result.id),
                reason: result
                    .reason
                    .unwrap_or_else(|| format!("{} is not enabled.", result.id.as_str())),
            })
            .collect(),
        implementations: Vec::new(),
    }
}

struct CapabilityResult {
    id: CapabilityId,
    state: WorkspaceAvailabilityState,
    limited: bool,
    reason: Option<String>,
    implementation: Option<String>,
}

impl CapabilityResult {
    fn is_enabled(&self) -> bool {
        self.state.is_enabled() && self.implementation.is_some()
    }
}

fn capability_result(
    authority: &DaemonCompatibilitySnapshot,
    id: CapabilityId,
) -> CapabilityResult {
    let Some(record) = authority.snapshot.capability(id) else {
        return CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Unknown,
            limited: false,
            reason: Some(format!("{} has no capability evidence.", id.as_str())),
            implementation: None,
        };
    };
    let implementation = authority
        .implementations
        .get(&id)
        .map(|implementation| implementation.id.clone());
    match &record.state {
        CapabilityState::Available if implementation.is_some() => CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Available,
            limited: false,
            reason: None,
            implementation,
        },
        CapabilityState::Available => CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Unknown,
            limited: false,
            reason: Some(format!(
                "{} is enabled but has no selected implementation.",
                id.as_str()
            )),
            implementation: None,
        },
        CapabilityState::AvailableWithLimitations { reason, .. } if implementation.is_some() => {
            CapabilityResult {
                id,
                state: WorkspaceAvailabilityState::AvailableWithLimitations,
                limited: true,
                reason: Some(reason.message.clone()),
                implementation,
            }
        }
        CapabilityState::AvailableWithLimitations { .. } => CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Unknown,
            limited: false,
            reason: Some(format!(
                "{} is limited but has no selected implementation.",
                id.as_str()
            )),
            implementation: None,
        },
        CapabilityState::Unavailable { reason } => CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Unavailable,
            limited: false,
            reason: Some(reason.message.clone()),
            implementation: None,
        },
        CapabilityState::Unknown { reason } => CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Unknown,
            limited: false,
            reason: Some(reason.message.clone()),
            implementation: None,
        },
        CapabilityState::Unsupported { reason } => CapabilityResult {
            id,
            state: WorkspaceAvailabilityState::Unsupported,
            limited: false,
            reason: Some(reason.message.clone()),
            implementation: None,
        },
    }
}

fn capability_names(capabilities: &[CapabilityId]) -> String {
    capabilities
        .iter()
        .map(|capability| capability.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Screen rendering is always locally reachable so unavailable workflows can
/// remain discoverable. Effect-producing actions inside each destination are
/// authorized separately by `workspace_effect_requirement`.
pub const fn workspace_screen_destination(screen: Screen) -> WorkspaceDestination {
    match screen {
        Screen::Dashboard => WorkspaceDestination::Dashboard,
        Screen::Tasks => WorkspaceDestination::Tasks,
        Screen::BuildHistory => WorkspaceDestination::BuildHistory,
        Screen::Dependencies => WorkspaceDestination::Dependencies,
        Screen::Signatures => WorkspaceDestination::Signatures,
        Screen::LayerRelationships => WorkspaceDestination::Layers,
        Screen::Recipes => WorkspaceDestination::Recipes,
        Screen::Packages => WorkspaceDestination::Packages,
        Screen::Images => WorkspaceDestination::Images,
        Screen::Sdk => WorkspaceDestination::Sdk,
        Screen::Testing => WorkspaceDestination::Testing,
        Screen::Security => WorkspaceDestination::Security,
        Screen::Qa => WorkspaceDestination::Qa,
        Screen::Layers => WorkspaceDestination::Layers,
        Screen::Configuration | Screen::Bbmask => WorkspaceDestination::Configuration,
        Screen::Maintenance => WorkspaceDestination::Maintenance,
        Screen::Logs => WorkspaceDestination::Logs,
        Screen::Errors => WorkspaceDestination::Errors,
        Screen::Help => WorkspaceDestination::Help,
        Screen::BuildEnvironment => WorkspaceDestination::BuildEnvironment,
        Screen::Compatibility => WorkspaceDestination::Compatibility,
        Screen::Settings => WorkspaceDestination::Settings,
    }
}

/// Capability summary for environment-backed functionality in a destination.
/// Navigation itself remains local and visible even when this requirement is
/// unsatisfied so the UI can explain the exact unavailable features.
pub fn workspace_destination_requirement(
    destination: WorkspaceDestination,
) -> WorkspaceEffectRequirement {
    use CapabilityId as Id;
    match destination {
        WorkspaceDestination::Dashboard
        | WorkspaceDestination::BuildHistory
        | WorkspaceDestination::Logs
        | WorkspaceDestination::Errors
        | WorkspaceDestination::ProjectProfiles
        | WorkspaceDestination::BuildEnvironment
        | WorkspaceDestination::Compatibility
        | WorkspaceDestination::Settings
        | WorkspaceDestination::Help => WorkspaceEffectRequirement::ClientLocal,
        WorkspaceDestination::Recipes => {
            WorkspaceEffectRequirement::one(Id::BitBakeRecipeInventory)
        }
        WorkspaceDestination::Layers => WorkspaceEffectRequirement::all_and_any(
            &[],
            &[Id::BitBakeLayerInventory, Id::BitBakeLayersShowLayers],
        ),
        WorkspaceDestination::Configuration => WorkspaceEffectRequirement::one(Id::BitBakeGetVar),
        WorkspaceDestination::Tasks => WorkspaceEffectRequirement::one(Id::BitBakeTaskList),
        WorkspaceDestination::Dependencies => WorkspaceEffectRequirement::all_and_any(
            &[],
            &[Id::BitBakeRecipeDependencies, Id::BitBakeDependencyGraph],
        ),
        WorkspaceDestination::Signatures => {
            WorkspaceEffectRequirement::all_and_any(&[], &[Id::BitBakeDumpSig, Id::BitBakeDiffSigs])
        }
        WorkspaceDestination::Packages => {
            WorkspaceEffectRequirement::all(&[Id::PkgDataGenerated, Id::PkgDataListPackages])
        }
        WorkspaceDestination::Images => WorkspaceEffectRequirement::all_and_any(
            &[],
            &[Id::BitBakeBuild, Id::RunQemu, Id::WicCreate],
        ),
        WorkspaceDestination::Sdk => WorkspaceEffectRequirement::all_and_any(
            &[],
            &[
                Id::SdkPopulate,
                Id::SdkExtensible,
                Id::SdkPublish,
                Id::SdkNativeTools,
            ],
        ),
        WorkspaceDestination::Testing => WorkspaceEffectRequirement::all_and_any(
            &[],
            &[
                Id::OeSelftest,
                Id::BitBakeSelftest,
                Id::TestImage,
                Id::TestSdk,
                Id::TestSdkExtensible,
                Id::Ptest,
                Id::ResultTool,
            ],
        ),
        WorkspaceDestination::Security => {
            WorkspaceEffectRequirement::all_and_any(&[], &[Id::CveCheck, Id::SpdxCreate])
        }
        WorkspaceDestination::Qa => {
            WorkspaceEffectRequirement::all_and_any(&[], &[Id::QaTask, Id::YoctoCheckLayer])
        }
        WorkspaceDestination::Devtool => WorkspaceEffectRequirement::all_and_any(
            &[],
            &[
                Id::DevtoolStatus,
                Id::DevtoolModify,
                Id::DevtoolUpdateRecipe,
                Id::DevtoolFinish,
                Id::DevtoolDeployTarget,
                Id::DevtoolReset,
                Id::DevtoolUpgrade,
            ],
        ),
        WorkspaceDestination::QemuWic => {
            WorkspaceEffectRequirement::all_and_any(&[], &[Id::RunQemu, Id::WicCreate])
        }
        WorkspaceDestination::Maintenance => WorkspaceEffectRequirement::all_and_any(
            &[],
            &[
                Id::SstateReadiness,
                Id::SstateCleanup,
                Id::PrservManagement,
                Id::LockedSignatures,
                Id::BuildHistoryCompare,
                Id::BuildCompare,
                Id::GitArchive,
            ],
        ),
        WorkspaceDestination::TerminalSessions => WorkspaceEffectRequirement::all_and_any(
            &[],
            &[Id::DevShell, Id::MenuConfig, Id::BitBakeServerSocket],
        ),
    }
}

pub fn workspace_effect_requirement(effect: &Effect) -> WorkspaceEffectRequirement {
    use CapabilityId as Id;
    use WorkspaceEffectRequirement as Requirement;

    match effect {
        Effect::PersistSettings
        | Effect::GenerateProjectProfile { .. }
        | Effect::VerifyBuildEnvironment { .. }
        | Effect::CloneBuildEnvironment(_)
        | Effect::OpenInEditor(_)
        | Effect::CopyToClipboard(_)
        | Effect::OpenWorkspaceEditor { .. }
        | Effect::LoadLayerBrowserDirectory { .. }
        | Effect::LoadLayerBrowserPreview(_)
        | Effect::OpenLayerBrowserEditor { .. }
        | Effect::GetImageArtifacts(_)
        | Effect::CancelImageArtifactOperation
        | Effect::GetSdkArtifacts(_)
        | Effect::CancelSdkArtifactOperation
        | Effect::CancelSignatureOperation
        | Effect::CancelPackageOperation
        | Effect::CancelSdkSession(_)
        | Effect::CancelTestSession(_)
        | Effect::ImportTestResults(_)
        | Effect::InspectTestJunitDestination { .. }
        | Effect::ExportTestJunit(_)
        | Effect::GetWicOutputs(_)
        | Effect::GetWicDevices(_)
        | Effect::LoadRecipeEditorFile(_)
        | Effect::SaveRecipeEditorFile { .. }
        | Effect::WriteConfigAssignment(_)
        | Effect::WriteBbmask(_) => Requirement::ClientLocal,

        Effect::Start(request) => build_request_requirement(request),
        Effect::Cancel => Requirement::one(Id::BitBakeCancellation),
        Effect::DevtoolModify(_) => Requirement::one(Id::DevtoolModify),
        Effect::DevtoolReset(_) => Requirement::one(Id::DevtoolReset),
        Effect::DevtoolUpdateRecipe(_) => Requirement::one(Id::DevtoolUpdateRecipe),
        Effect::DevtoolFinish(_) => Requirement::one(Id::DevtoolFinish),
        Effect::DevtoolDeploy(_) => Requirement::one(Id::DevtoolDeployTarget),
        Effect::InspectDevtoolStatus(_) => Requirement::one(Id::DevtoolStatus),
        Effect::GetDependencies(_) => Requirement::one(Id::BitBakeRecipeDependencies),
        Effect::GetSignatureDump(_) => Requirement::one(Id::BitBakeDumpSig),
        Effect::CompareSignatures(_) => Requirement::one(Id::BitBakeDiffSigs),
        Effect::GetPackageInventory(_) => {
            Requirement::all(&[Id::PkgDataGenerated, Id::PkgDataListPackages])
        }
        Effect::GetPackageDetail(_) => Requirement::all(&[
            Id::PkgDataGenerated,
            Id::PkgDataPackageInfo,
            Id::PkgDataListPackageFiles,
            Id::PkgDataReadValue,
        ]),
        Effect::InspectSdkTools => Requirement::probe(&[Id::SdkPublish, Id::SdkNativeTools]),
        Effect::StartSdkSession { operation, .. } => match operation {
            SdkOperation::Publish(_) => Requirement::one(Id::SdkPublish),
            SdkOperation::Native(_) => Requirement::one(Id::SdkNativeTools),
        },
        Effect::InspectTestCapability => Requirement::probe(&[
            Id::OeSelftest,
            Id::BitBakeSelftest,
            Id::TestImage,
            Id::TestSdk,
            Id::TestSdkExtensible,
            Id::Ptest,
        ]),
        Effect::StartTestSession { operation, .. } => test_operation_requirement(operation),
        Effect::StartTestBuildSession { family, .. } => test_family_requirement(*family, true),
        Effect::InspectResultToolCapability => Requirement::probe(&[Id::ResultTool]),
        Effect::CompareTestResults(_) => Requirement::one(Id::ResultTool),
        Effect::Security(effect) => security_effect_requirement(effect),
        Effect::Qa(effect) => qa_effect_requirement(effect),
        Effect::Maintenance(effect) => maintenance_effect_requirement(effect),
        Effect::InspectQemuCapability => Requirement::probe(&[Id::RunQemu]),
        Effect::StartQemuSession { .. } => Requirement::one(Id::RunQemu),
        Effect::CancelQemuSession(_) => Requirement::ClientLocal,
        Effect::InspectWicCapability => Requirement::probe(&[Id::WicCreate]),
        Effect::StartWicSession { operation, .. } => match operation {
            WicOperation::Create(_) => Requirement::one(Id::WicCreate),
            // Device writing is a host-local artifact operation and does not
            // imply that the connected Yocto environment exposes `wic create`.
            WicOperation::Write(_) => Requirement::ClientLocal,
        },
        Effect::CancelWicSession(_) => Requirement::ClientLocal,
        Effect::GetRecipeMetadata(_) => Requirement::one(Id::BitBakeRecipeMetadata),
        Effect::GetVariable(_) => Requirement::one(Id::BitBakeGetVar),
        Effect::GetLayerRelationships => Requirement::one(Id::BitBakeLayerRelationships),
    }
}

fn build_request_requirement(request: &BuildRequest) -> WorkspaceEffectRequirement {
    use CapabilityId as Id;
    let task = request.task.as_deref();
    match task {
        Some("populate_sdk") => {
            WorkspaceEffectRequirement::all(&[Id::BitBakeBuild, Id::SdkPopulate])
        }
        Some("populate_sdk_ext") => {
            WorkspaceEffectRequirement::all(&[Id::BitBakeBuild, Id::SdkExtensible])
        }
        Some("testsdk") => WorkspaceEffectRequirement::all(&[Id::BitBakeBuild, Id::TestSdk]),
        Some("testsdkext") => {
            WorkspaceEffectRequirement::all(&[Id::BitBakeBuild, Id::TestSdkExtensible])
        }
        Some("testimage") => WorkspaceEffectRequirement::all(&[Id::BitBakeBuild, Id::TestImage]),
        Some("cve_check") => WorkspaceEffectRequirement::all(&[Id::BitBakeBuild, Id::CveCheck]),
        Some("create_spdx" | "create_recipe_sbom" | "create_rootfs_sbom") => {
            WorkspaceEffectRequirement::all(&[Id::BitBakeBuild, Id::SpdxCreate])
        }
        _ if request.force => {
            WorkspaceEffectRequirement::all(&[Id::BitBakeBuild, Id::BitBakeForceTask])
        }
        _ => WorkspaceEffectRequirement::one(Id::BitBakeBuild),
    }
}

fn test_operation_requirement(operation: &TestOperation) -> WorkspaceEffectRequirement {
    match operation {
        TestOperation::Selftest(request) => test_family_requirement(request.family, false),
        TestOperation::Build { family, .. } => test_family_requirement(*family, true),
    }
}

fn test_family_requirement(family: TestFamily, requires_build: bool) -> WorkspaceEffectRequirement {
    use CapabilityId as Id;
    let capability = match family {
        TestFamily::OeSelftest => Id::OeSelftest,
        TestFamily::BitbakeSelftest => Id::BitBakeSelftest,
        TestFamily::TestImage => Id::TestImage,
        TestFamily::TestSdk => Id::TestSdk,
        TestFamily::TestSdkExt => Id::TestSdkExtensible,
        TestFamily::Ptest => Id::Ptest,
    };
    if requires_build {
        WorkspaceEffectRequirement::all(&[Id::BitBakeBuild, capability])
    } else {
        WorkspaceEffectRequirement::one(capability)
    }
}

fn security_effect_requirement(effect: &SecurityEffect) -> WorkspaceEffectRequirement {
    use CapabilityId as Id;
    match effect {
        SecurityEffect::InspectCapability => {
            WorkspaceEffectRequirement::probe(&[Id::CveCheck, Id::SpdxCreate])
        }
        SecurityEffect::StartBuild { request, .. } => build_request_requirement(request),
        SecurityEffect::StartPackageMap { .. } => {
            WorkspaceEffectRequirement::all(&[Id::PkgDataGenerated, Id::PkgDataLookupPackage])
        }
        SecurityEffect::CancelSession(_) => WorkspaceEffectRequirement::ClientLocal,
        SecurityEffect::ImportReports(_)
        | SecurityEffect::OpenPath(_)
        | SecurityEffect::OpenUrl(_) => WorkspaceEffectRequirement::ClientLocal,
    }
}

fn qa_effect_requirement(effect: &QaEffect) -> WorkspaceEffectRequirement {
    use CapabilityId as Id;
    match effect {
        QaEffect::InspectCapability { .. } => WorkspaceEffectRequirement::probe(&[Id::QaTask]),
        QaEffect::StartBuild { .. } => {
            WorkspaceEffectRequirement::all(&[Id::BitBakeBuild, Id::QaTask])
        }
        QaEffect::CancelBuild { .. } => WorkspaceEffectRequirement::ClientLocal,
        QaEffect::InspectLayerCapability => {
            WorkspaceEffectRequirement::probe(&[Id::YoctoCheckLayer])
        }
        QaEffect::StartLayerCheck { .. } => WorkspaceEffectRequirement::one(Id::YoctoCheckLayer),
        QaEffect::CancelLayerCheck(_) => WorkspaceEffectRequirement::ClientLocal,
        QaEffect::ImportReports(_)
        | QaEffect::OpenReport(_)
        | QaEffect::OpenProvider(_)
        | QaEffect::OpenSource(_)
        | QaEffect::OpenLayerRoot(_) => WorkspaceEffectRequirement::ClientLocal,
    }
}

fn maintenance_effect_requirement(effect: &MaintenanceEffect) -> WorkspaceEffectRequirement {
    use CapabilityId as Id;
    match effect {
        MaintenanceEffect::InspectCapability { .. } => WorkspaceEffectRequirement::probe(&[
            Id::SstateReadiness,
            Id::SstateCleanup,
            Id::LockedSignatures,
            Id::BuildHistoryCompare,
            Id::BuildCompare,
            Id::GitArchive,
        ]),
        MaintenanceEffect::InspectServices { .. } => {
            WorkspaceEffectRequirement::probe(&[Id::HashservDiagnostics, Id::PrservDiagnostics])
        }
        MaintenanceEffect::PreviewReadiness { .. } => {
            WorkspaceEffectRequirement::one(Id::SstateReadiness)
        }
        MaintenanceEffect::PreviewCleanup { .. } => {
            WorkspaceEffectRequirement::one(Id::SstateCleanup)
        }
        MaintenanceEffect::PreviewPrService { .. } => {
            WorkspaceEffectRequirement::one(Id::PrservManagement)
        }
        MaintenanceEffect::PreviewLockedSignatureCache { .. } => {
            WorkspaceEffectRequirement::one(Id::LockedSignatures)
        }
        MaintenanceEffect::PreviewBuildHistoryComparison { .. } => {
            WorkspaceEffectRequirement::one(Id::BuildHistoryCompare)
        }
        MaintenanceEffect::PreviewGitArchive { .. } => {
            WorkspaceEffectRequirement::one(Id::GitArchive)
        }
        MaintenanceEffect::StartOperation { preview, .. } => match &preview.operation {
            MaintenanceOperation::SstateReadiness(_) => {
                WorkspaceEffectRequirement::one(Id::SstateReadiness)
            }
            MaintenanceOperation::SstateCleanup(_) => {
                WorkspaceEffectRequirement::one(Id::SstateCleanup)
            }
            MaintenanceOperation::PrService(_) => {
                WorkspaceEffectRequirement::one(Id::PrservManagement)
            }
            MaintenanceOperation::LockedSignatureCache(_) => {
                WorkspaceEffectRequirement::one(Id::LockedSignatures)
            }
            MaintenanceOperation::BuildHistoryComparison(_) => {
                WorkspaceEffectRequirement::one(Id::BuildHistoryCompare)
            }
            MaintenanceOperation::BuildCompare(_) => {
                WorkspaceEffectRequirement::one(Id::BuildCompare)
            }
            MaintenanceOperation::GitArchive(_) => WorkspaceEffectRequirement::one(Id::GitArchive),
        },
        MaintenanceEffect::CancelOperation(_) => WorkspaceEffectRequirement::ClientLocal,
        MaintenanceEffect::OpenEvidence(_) | MaintenanceEffect::Navigate(_) => {
            WorkspaceEffectRequirement::ClientLocal
        }
    }
}

pub fn workspace_dialog_requirement(dialog: &Dialog) -> WorkspaceEffectRequirement {
    use CapabilityId as Id;
    match dialog {
        Dialog::BuildEnvironmentCloneEditor(_)
        | Dialog::BuildEnvironmentCloneReview(_)
        | Dialog::BuildEnvironmentEditor(_)
        | Dialog::ThemePicker { .. }
        | Dialog::BuildCompletion
        | Dialog::WicDevicePicker(_)
        | Dialog::WicWritePhrase(_)
        | Dialog::WicWriteConfirmation(_)
        | Dialog::WicCancellationConfirmation { .. }
        | Dialog::SdkCancellationConfirmation(_)
        | Dialog::TestCancellationConfirmation(_)
        | Dialog::TestResultImport(_)
        | Dialog::TestResultImportTomlEditor { .. }
        | Dialog::TestJunitExport(_)
        | Dialog::TestJunitTomlEditor { .. }
        | Dialog::TestJunitExportConfirmation(_)
        | Dialog::RecipeTaskPicker(_)
        | Dialog::RecipeTaskLogPicker(_)
        | Dialog::RecipePatchPicker(_)
        | Dialog::ConfigSourcePicker(_)
        | Dialog::ConfigScopePicker(_)
        | Dialog::ConfigComparison(_)
        | Dialog::ConfigEdit { .. }
        | Dialog::ConfigEditConfirmation(_)
        | Dialog::BbmaskEdit(_)
        | Dialog::BbmaskConfirmation(_)
        | Dialog::RecipeEditor(_)
        | Dialog::QuitConfirmation => WorkspaceEffectRequirement::ClientLocal,
        Dialog::BuildOptions | Dialog::BuildTarget { .. } => {
            WorkspaceEffectRequirement::one(Id::BitBakeBuild)
        }
        Dialog::ImagePicker(_) => WorkspaceEffectRequirement::one(Id::BitBakeBuild),
        Dialog::QemuLaunch(_) | Dialog::QemuLaunchConfirmation(_) => {
            WorkspaceEffectRequirement::one(Id::RunQemu)
        }
        Dialog::QemuCancellationConfirmation(_) => WorkspaceEffectRequirement::ClientLocal,
        Dialog::WicCreate(_)
        | Dialog::WicCreateTomlEditor { .. }
        | Dialog::WicCreateConfirmation(_) => WorkspaceEffectRequirement::one(Id::WicCreate),
        Dialog::SdkBuildConfirmation(preview) => build_request_requirement(&preview.request),
        Dialog::SdkPublish(_)
        | Dialog::SdkPublishTomlEditor(_)
        | Dialog::SdkPublishConfirmation(_) => WorkspaceEffectRequirement::one(Id::SdkPublish),
        Dialog::SdkNative(_)
        | Dialog::SdkNativeTomlEditor(_)
        | Dialog::SdkNativeConfirmation(_) => WorkspaceEffectRequirement::one(Id::SdkNativeTools),
        Dialog::TestLaunch(dialog) => test_family_requirement(
            dialog.draft.family,
            !matches!(
                dialog.draft.family,
                TestFamily::OeSelftest | TestFamily::BitbakeSelftest
            ),
        ),
        Dialog::TestLaunchTomlEditor { family, .. } => test_family_requirement(
            *family,
            !matches!(
                *family,
                TestFamily::OeSelftest | TestFamily::BitbakeSelftest
            ),
        ),
        Dialog::TestLaunchConfirmation(preview) => match preview {
            TestLaunchPreview::Selftest(request) => test_family_requirement(request.family, false),
            TestLaunchPreview::Build { family, .. } => test_family_requirement(*family, true),
        },
        Dialog::TestComparison(_) | Dialog::TestComparisonTomlEditor { .. } => {
            WorkspaceEffectRequirement::one(Id::ResultTool)
        }
        Dialog::TestComparisonConfirmation(_) => WorkspaceEffectRequirement::one(Id::ResultTool),
        Dialog::Security(dialog) => security_dialog_requirement(dialog),
        Dialog::Qa(dialog) => qa_dialog_requirement(dialog),
        Dialog::Maintenance(dialog) => maintenance_dialog_requirement(dialog),
        Dialog::RecipeTaskConfirmation(request) => build_request_requirement(request),
        Dialog::SignatureTaskPicker(_) => WorkspaceEffectRequirement::one(Id::BitBakeDumpSig),
        Dialog::DevtoolModifyConfirmation(_) => WorkspaceEffectRequirement::one(Id::DevtoolModify),
        Dialog::DevtoolResetConfirmation(_) => WorkspaceEffectRequirement::one(Id::DevtoolReset),
        Dialog::DevtoolUpdateConfirmation(_) => {
            WorkspaceEffectRequirement::one(Id::DevtoolUpdateRecipe)
        }
        Dialog::DevtoolFinishPicker(_) | Dialog::DevtoolFinishConfirmation(_) => {
            WorkspaceEffectRequirement::one(Id::DevtoolFinish)
        }
        Dialog::DevtoolDeploy(_) | Dialog::DevtoolDeployConfirmation(_) => {
            WorkspaceEffectRequirement::one(Id::DevtoolDeployTarget)
        }
    }
}

fn security_dialog_requirement(dialog: &SecurityDialog) -> WorkspaceEffectRequirement {
    use CapabilityId as Id;
    match dialog {
        SecurityDialog::Operation(preview) => match &preview.operation {
            SecurityOperation::CveCheck(request) | SecurityOperation::SbomBuild(request) => {
                build_request_requirement(request)
            }
            SecurityOperation::PackageMap { .. } => {
                WorkspaceEffectRequirement::all(&[Id::PkgDataGenerated, Id::PkgDataLookupPackage])
            }
        },
        SecurityDialog::Import { .. } | SecurityDialog::Cancellation(_) => {
            WorkspaceEffectRequirement::ClientLocal
        }
    }
}

fn qa_dialog_requirement(dialog: &QaDialog) -> WorkspaceEffectRequirement {
    use CapabilityId as Id;
    match dialog {
        QaDialog::Operation(_) => WorkspaceEffectRequirement::all(&[Id::BitBakeBuild, Id::QaTask]),
        QaDialog::LayerOperation(_) => WorkspaceEffectRequirement::one(Id::YoctoCheckLayer),
        QaDialog::Import { .. }
        | QaDialog::Cancellation { .. }
        | QaDialog::LayerCancellation(_) => WorkspaceEffectRequirement::ClientLocal,
    }
}

fn maintenance_dialog_requirement(dialog: &MaintenanceDialog) -> WorkspaceEffectRequirement {
    use CapabilityId as Id;
    match dialog {
        MaintenanceDialog::ReadinessToml { .. } | MaintenanceDialog::ReadinessForm(_) => {
            WorkspaceEffectRequirement::one(Id::SstateReadiness)
        }
        MaintenanceDialog::CleanupToml { .. } | MaintenanceDialog::CleanupForm(_) => {
            WorkspaceEffectRequirement::one(Id::SstateCleanup)
        }
        MaintenanceDialog::PrServiceToml { .. } | MaintenanceDialog::PrServiceForm(_) => {
            WorkspaceEffectRequirement::one(Id::PrservManagement)
        }
        MaintenanceDialog::LockedCacheToml { .. } | MaintenanceDialog::LockedCacheForm(_) => {
            WorkspaceEffectRequirement::one(Id::LockedSignatures)
        }
        MaintenanceDialog::BuildHistoryToml { .. } | MaintenanceDialog::BuildHistoryForm(_) => {
            WorkspaceEffectRequirement::one(Id::BuildHistoryCompare)
        }
        MaintenanceDialog::GitArchiveToml { .. } | MaintenanceDialog::GitArchiveForm(_) => {
            WorkspaceEffectRequirement::one(Id::GitArchive)
        }
        MaintenanceDialog::Confirm(preview)
        | MaintenanceDialog::CleanupPhrase { preview, .. }
        | MaintenanceDialog::ConfirmNetworkPush(preview) => match &preview.operation {
            MaintenanceOperation::SstateReadiness(_) => {
                WorkspaceEffectRequirement::one(Id::SstateReadiness)
            }
            MaintenanceOperation::SstateCleanup(_) => {
                WorkspaceEffectRequirement::one(Id::SstateCleanup)
            }
            MaintenanceOperation::PrService(_) => {
                WorkspaceEffectRequirement::one(Id::PrservManagement)
            }
            MaintenanceOperation::LockedSignatureCache(_) => {
                WorkspaceEffectRequirement::one(Id::LockedSignatures)
            }
            MaintenanceOperation::BuildHistoryComparison(_) => {
                WorkspaceEffectRequirement::one(Id::BuildHistoryCompare)
            }
            MaintenanceOperation::BuildCompare(_) => {
                WorkspaceEffectRequirement::one(Id::BuildCompare)
            }
            MaintenanceOperation::GitArchive(_) => WorkspaceEffectRequirement::one(Id::GitArchive),
        },
        MaintenanceDialog::ConfirmCancellation(_) => WorkspaceEffectRequirement::ClientLocal,
    }
}

pub fn authorize_workspace_effect(
    app: &App,
    effect: &Effect,
) -> Result<WorkspaceAvailability, WorkspaceEffectDenied> {
    let availability = app
        .workspace_compatibility
        .availability(&workspace_effect_requirement(effect));
    if availability.is_enabled() {
        Ok(availability)
    } else {
        Err(WorkspaceEffectDenied { availability })
    }
}

/// Capability-aware reducer boundary. If an action attempts to emit an
/// unavailable environment effect, preparation mutations are rolled back and
/// only the exact denial notice is retained.
pub fn update_with_workspace_authority(app: &mut App, action: crate::Action) -> Option<Effect> {
    let before = app.clone();
    let effect = crate::update(app, action)?;
    match authorize_workspace_effect(app, &effect) {
        Ok(_) => Some(effect),
        Err(error) => {
            *app = before;
            app.notification = Some(error.reason());
            None
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("workspace effect is unavailable")]
pub struct WorkspaceEffectDenied {
    pub availability: WorkspaceAvailability,
}

impl WorkspaceEffectDenied {
    pub fn reason(&self) -> String {
        self.availability
            .issues
            .iter()
            .map(|issue| issue.reason.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub fn install_workspace_compatibility(
    app: &mut App,
    authority: DaemonCompatibilitySnapshot,
) -> Result<WorkspaceRevalidation, WorkspaceCompatibilityError> {
    let install = app.workspace_compatibility.install(authority)?;
    app.compatibility_ui
        .reconcile(app.workspace_compatibility.authority());
    if install == WorkspaceSnapshotInstall::Unchanged {
        return Ok(WorkspaceRevalidation {
            install,
            closed_dialog: false,
            reason: None,
        });
    }
    Ok(revalidate_workspace_dialog(app, install))
}

pub fn invalidate_workspace_compatibility(app: &mut App) -> WorkspaceRevalidation {
    app.workspace_compatibility.invalidate();
    app.compatibility_ui.reconcile(None);
    revalidate_workspace_dialog(app, WorkspaceSnapshotInstall::Invalidated)
}

fn revalidate_workspace_dialog(
    app: &mut App,
    install: WorkspaceSnapshotInstall,
) -> WorkspaceRevalidation {
    let unavailable = app.active_dialog().and_then(|dialog| {
        let availability = app
            .workspace_compatibility
            .availability(&workspace_dialog_requirement(dialog));
        (!availability.is_enabled()).then_some(availability)
    });
    let reason = unavailable.as_ref().map(|availability| {
        availability
            .issues
            .iter()
            .map(|issue| issue.reason.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    });
    if unavailable.is_some() {
        crate::close_dialog(app);
        app.notification = reason
            .as_ref()
            .map(|reason| format!("Action closed after environment capability update: {reason}"));
        crate::synchronize_focus(app);
    }
    WorkspaceRevalidation {
        install,
        closed_dialog: unavailable.is_some(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BuildRequest, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityImplementation, CapabilityImplementationKind, CapabilityReason, CapabilityRecord,
        CapabilitySnapshot, CapabilityState, DaemonCompatibilitySnapshot, Effect, VariableIdentity,
        YoctoEnvironmentIdentity,
    };
    use std::collections::BTreeMap;

    fn reason(message: &str) -> CapabilityReason {
        CapabilityReason::new("test.workspace", message, None).unwrap()
    }

    fn authority(
        generation: u64,
        records: Vec<(CapabilityId, CapabilityState, Option<&str>)>,
    ) -> DaemonCompatibilitySnapshot {
        let mut capabilities = records
            .iter()
            .map(|(id, state, _)| CapabilityRecord {
                id: *id,
                state: state.clone(),
                evidence: match state {
                    CapabilityState::Available
                    | CapabilityState::AvailableWithLimitations { .. } => {
                        vec![CapabilityEvidence {
                            kind: CapabilityEvidenceKind::DirectProbe,
                            outcome: CapabilityEvidenceOutcome::Positive,
                            subject: id.as_str().into(),
                            detail: "positive workspace fixture evidence".into(),
                            argv: vec!["fixture".into()],
                        }]
                    }
                    CapabilityState::Unavailable { .. } => vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: CapabilityEvidenceOutcome::Negative,
                        subject: id.as_str().into(),
                        detail: "negative workspace fixture evidence".into(),
                        argv: vec!["fixture".into()],
                    }],
                    CapabilityState::Unknown { .. } | CapabilityState::Unsupported { .. } => {
                        Vec::new()
                    }
                },
            })
            .collect::<Vec<_>>();
        capabilities.sort_by_key(|record| record.id);
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation,
                environment: YoctoEnvironmentIdentity::default(),
                capabilities,
            },
            implementations: records
                .into_iter()
                .filter_map(|(id, _, implementation)| {
                    implementation.map(|implementation| {
                        (
                            id,
                            CapabilityImplementation {
                                id: implementation.into(),
                                kind: CapabilityImplementationKind::Command,
                            },
                        )
                    })
                })
                .collect::<BTreeMap<_, _>>(),
        }
        .normalize()
        .unwrap()
    }

    #[test]
    fn compatibility_workspace_catalog_covers_every_screen_and_named_destination() {
        assert_eq!(WorkspaceDestination::ALL.len(), 25);
        for screen in [
            Screen::Dashboard,
            Screen::Tasks,
            Screen::BuildHistory,
            Screen::Dependencies,
            Screen::Signatures,
            Screen::LayerRelationships,
            Screen::Recipes,
            Screen::Packages,
            Screen::Images,
            Screen::Sdk,
            Screen::Testing,
            Screen::Security,
            Screen::Qa,
            Screen::Layers,
            Screen::Configuration,
            Screen::Bbmask,
            Screen::Maintenance,
            Screen::Logs,
            Screen::Errors,
            Screen::Help,
            Screen::BuildEnvironment,
            Screen::Compatibility,
            Screen::Settings,
        ] {
            let _ = workspace_screen_destination(screen);
        }
        assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::Devtool));
        assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::QemuWic));
        assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::ProjectProfiles));
        assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::TerminalSessions));
        assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::Compatibility));
        for destination in WorkspaceDestination::ALL {
            let _ = workspace_destination_requirement(destination);
        }
    }

    #[test]
    fn compatibility_workspace_catalog_classifies_local_single_all_and_alternative_effects() {
        assert_eq!(
            workspace_effect_requirement(&Effect::CopyToClipboard("value".into())),
            WorkspaceEffectRequirement::ClientLocal
        );
        assert_eq!(
            workspace_effect_requirement(&Effect::GetVariable(VariableIdentity {
                name: "MACHINE".into(),
                recipe: None,
            })),
            WorkspaceEffectRequirement::one(CapabilityId::BitBakeGetVar)
        );
        assert_eq!(
            workspace_effect_requirement(&Effect::Start(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: Some("populate_sdk_ext".into()),
                force: false,
            })),
            WorkspaceEffectRequirement::all(&[
                CapabilityId::BitBakeBuild,
                CapabilityId::SdkExtensible,
            ])
        );
        assert_eq!(
            workspace_destination_requirement(WorkspaceDestination::QemuWic),
            WorkspaceEffectRequirement::all_and_any(
                &[],
                &[CapabilityId::RunQemu, CapabilityId::WicCreate],
            )
        );
    }

    #[test]
    fn compatibility_workspace_catalog_marks_environment_inspection_daemon_owned() {
        assert_eq!(
            workspace_effect_requirement(&Effect::InspectQemuCapability),
            WorkspaceEffectRequirement::probe(&[CapabilityId::RunQemu])
        );
        assert_eq!(
            workspace_effect_requirement(&Effect::InspectSdkTools),
            WorkspaceEffectRequirement::probe(&[
                CapabilityId::SdkPublish,
                CapabilityId::SdkNativeTools,
            ])
        );
    }

    #[test]
    fn compatibility_workspace_catalog_new_external_behaviors_have_exact_probe_policy() {
        let catalog = crate::CapabilityCatalog::builtin();
        for id in [
            CapabilityId::SdkPublish,
            CapabilityId::SdkNativeTools,
            CapabilityId::BitBakeSelftest,
            CapabilityId::TestImage,
            CapabilityId::TestSdk,
            CapabilityId::TestSdkExtensible,
            CapabilityId::Ptest,
            CapabilityId::QaTask,
            CapabilityId::BuildHistoryCompare,
            CapabilityId::SstateReadiness,
            CapabilityId::SstateCleanup,
            CapabilityId::PrservManagement,
            CapabilityId::BuildCompare,
            CapabilityId::GitArchive,
        ] {
            let entry = catalog
                .entry(id)
                .expect("workspace capability is cataloged");
            assert!(!entry.probes.is_empty(), "{id} lacks probe policy");
            assert!(
                !entry.required_tools.is_empty() || !entry.required_metadata.is_empty(),
                "{id} lacks an authoritative requirement"
            );
            assert!(!entry.preferred.id.is_empty(), "{id} lacks implementation");
        }
        assert_eq!(
            catalog
                .entry(CapabilityId::SdkPublish)
                .unwrap()
                .required_tools,
            [crate::CapabilityToolId::OePublishSdk]
        );
        assert_eq!(
            catalog
                .entry(CapabilityId::BitBakeSelftest)
                .unwrap()
                .required_tools,
            [crate::CapabilityToolId::BitBakeSelftest]
        );
    }

    #[test]
    fn compatibility_workspace_model_projects_full_limited_all_and_any_requirements() {
        let authority = authority(
            1,
            vec![
                (
                    CapabilityId::BitBakeBuild,
                    CapabilityState::Available,
                    Some("tinfoil.build"),
                ),
                (
                    CapabilityId::SdkExtensible,
                    CapabilityState::AvailableWithLimitations {
                        reason: reason("legacy extensible SDK task is selected"),
                        limitations: vec!["legacy task adapter".into()],
                    },
                    Some("bitbake.populate_sdk_ext"),
                ),
                (
                    CapabilityId::RunQemu,
                    CapabilityState::Unavailable {
                        reason: reason("runqemu was not detected"),
                    },
                    None,
                ),
                (
                    CapabilityId::WicCreate,
                    CapabilityState::Available,
                    Some("wic.create.argv"),
                ),
            ],
        );
        let limited = workspace_requirement_availability(
            Some(&authority),
            &WorkspaceEffectRequirement::all(&[
                CapabilityId::BitBakeBuild,
                CapabilityId::SdkExtensible,
            ]),
        );
        assert_eq!(
            limited.state,
            WorkspaceAvailabilityState::AvailableWithLimitations
        );
        assert!(limited.issues[0].reason.contains("legacy extensible"));
        assert_eq!(limited.implementations.len(), 2);

        let alternative = workspace_requirement_availability(
            Some(&authority),
            &WorkspaceEffectRequirement::all_and_any(
                &[],
                &[CapabilityId::RunQemu, CapabilityId::WicCreate],
            ),
        );
        assert_eq!(alternative.state, WorkspaceAvailabilityState::Available);
        assert_eq!(
            alternative.implementations,
            vec![(CapabilityId::WicCreate, "wic.create.argv".into())]
        );
    }

    #[test]
    fn compatibility_workspace_model_absent_unknown_unsupported_and_missing_all_fail_closed() {
        let absent = workspace_requirement_availability(
            None,
            &WorkspaceEffectRequirement::all(&[
                CapabilityId::BitBakeBuild,
                CapabilityId::SdkPopulate,
            ]),
        );
        assert_eq!(absent.state, WorkspaceAvailabilityState::Unknown);
        assert_eq!(absent.issues.len(), 2);

        let authority = authority(
            2,
            vec![
                (
                    CapabilityId::BitBakeBuild,
                    CapabilityState::Unavailable {
                        reason: reason("build API was rejected"),
                    },
                    None,
                ),
                (
                    CapabilityId::SdkPopulate,
                    CapabilityState::Unsupported {
                        reason: reason("standard SDK workflow is intentionally unsupported"),
                    },
                    None,
                ),
                (
                    CapabilityId::RunQemu,
                    CapabilityState::Unknown {
                        reason: reason("runqemu probe timed out"),
                    },
                    None,
                ),
            ],
        );
        let all = workspace_requirement_availability(
            Some(&authority),
            &WorkspaceEffectRequirement::all(&[
                CapabilityId::BitBakeBuild,
                CapabilityId::SdkPopulate,
            ]),
        );
        assert_eq!(all.state, WorkspaceAvailabilityState::Unavailable);
        assert_eq!(all.issues.len(), 2);
        assert!(
            all.issues
                .iter()
                .any(|issue| issue.reason.contains("build API"))
        );
        let unknown = workspace_requirement_availability(
            Some(&authority),
            &WorkspaceEffectRequirement::one(CapabilityId::RunQemu),
        );
        assert_eq!(unknown.state, WorkspaceAvailabilityState::Unknown);
        assert!(unknown.issues[0].reason.contains("timed out"));
        let unsupported = workspace_requirement_availability(
            Some(&authority),
            &WorkspaceEffectRequirement::one(CapabilityId::SdkPopulate),
        );
        assert_eq!(unsupported.state, WorkspaceAvailabilityState::Unsupported);
    }

    #[test]
    fn compatibility_workspace_model_snapshot_install_is_monotonic_and_conflict_safe() {
        let current = authority(
            4,
            vec![(
                CapabilityId::BitBakeBuild,
                CapabilityState::Available,
                Some("tinfoil.build"),
            )],
        );
        let mut state = WorkspaceCompatibilityState::default();
        assert_eq!(
            state.install(current.clone()).unwrap(),
            WorkspaceSnapshotInstall::Replaced
        );
        assert_eq!(
            state.install(current).unwrap(),
            WorkspaceSnapshotInstall::Unchanged
        );
        assert!(matches!(
            state.install(authority(
                3,
                vec![(
                    CapabilityId::BitBakeBuild,
                    CapabilityState::Available,
                    Some("tinfoil.build")
                )]
            )),
            Err(WorkspaceCompatibilityError::StaleGeneration { .. })
        ));
        assert!(matches!(
            state.install(authority(
                4,
                vec![(
                    CapabilityId::BitBakeBuild,
                    CapabilityState::Unavailable {
                        reason: reason("same generation conflict")
                    },
                    None
                )]
            )),
            Err(WorkspaceCompatibilityError::ConflictingGeneration(4))
        ));
    }

    #[test]
    fn compatibility_workspace_model_snapshot_change_revalidates_dialog_and_effect() {
        let mut app = App::new(10, 1_000);
        app.navigator_selection = 3;
        app.dialogs.push_front(Dialog::BuildOptions);
        app.focus = crate::FocusTarget::Dialog;
        app.focus_return = Some(crate::FocusTarget::Inspector);
        let available = authority(
            1,
            vec![(
                CapabilityId::BitBakeBuild,
                CapabilityState::Available,
                Some("tinfoil.build"),
            )],
        );
        let retained = install_workspace_compatibility(&mut app, available).unwrap();
        assert!(!retained.closed_dialog);
        assert!(matches!(app.active_dialog(), Some(Dialog::BuildOptions)));
        let start = Effect::Start(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        });
        assert!(authorize_workspace_effect(&app, &start).is_ok());

        let unavailable = authority(
            2,
            vec![(
                CapabilityId::BitBakeBuild,
                CapabilityState::Unavailable {
                    reason: reason("connected backend cannot build"),
                },
                None,
            )],
        );
        let revalidated = install_workspace_compatibility(&mut app, unavailable).unwrap();
        assert!(revalidated.closed_dialog);
        assert!(app.active_dialog().is_none());
        assert_eq!(app.focus, crate::FocusTarget::Inspector);
        assert_eq!(app.navigator_selection, 3);
        let denied = authorize_workspace_effect(&app, &start).unwrap_err();
        assert!(denied.reason().contains("cannot build"));
        assert!(
            authorize_workspace_effect(&app, &Effect::CopyToClipboard("still local".into()))
                .is_ok()
        );
    }

    #[test]
    fn compatibility_workspace_model_invalidation_closes_environment_dialog_but_keeps_local_one() {
        let mut app = App::new(10, 1_000);
        install_workspace_compatibility(
            &mut app,
            authority(
                1,
                vec![(
                    CapabilityId::BitBakeBuild,
                    CapabilityState::Available,
                    Some("tinfoil.build"),
                )],
            ),
        )
        .unwrap();
        app.dialogs.push_front(Dialog::BuildOptions);
        let invalidated = invalidate_workspace_compatibility(&mut app);
        assert_eq!(invalidated.install, WorkspaceSnapshotInstall::Invalidated);
        assert!(invalidated.closed_dialog);

        app.dialogs.push_front(Dialog::QuitConfirmation);
        let local = invalidate_workspace_compatibility(&mut app);
        assert!(!local.closed_dialog);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::QuitConfirmation)
        ));
    }

    #[test]
    fn compatibility_workspace_model_unavailable_effect_is_not_emitted_or_partially_applied() {
        let mut app = App::new(10, 1_000);
        let request = BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        };
        app.dialogs
            .push_front(Dialog::RecipeTaskConfirmation(request));
        app.focus = crate::FocusTarget::Dialog;
        let before_build = app.build.clone();

        assert!(
            update_with_workspace_authority(&mut app, crate::Action::ConfirmRecipeTask).is_none()
        );
        assert_eq!(app.build, before_build);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::RecipeTaskConfirmation(_))
        ));
        assert!(
            app.notification
                .as_deref()
                .unwrap()
                .contains("current environment capability snapshot")
        );
    }
}
