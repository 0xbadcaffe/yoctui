use crate::{
    BuildRequest, CapabilityId, Effect, MaintenanceEffect, MaintenanceOperation, QaEffect, Screen,
    SdkOperation, SecurityEffect, TestFamily, TestOperation, WicOperation,
};

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
    Settings,
    Help,
}

impl WorkspaceDestination {
    pub const ALL: [Self; 24] = [
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

impl WorkspaceEffectRequirement {
    fn one(id: CapabilityId) -> Self {
        Self::Capabilities {
            all: vec![id],
            any: Vec::new(),
        }
    }

    fn all(ids: &[CapabilityId]) -> Self {
        Self::Capabilities {
            all: ids.to_vec(),
            any: Vec::new(),
        }
    }

    fn all_and_any(all: &[CapabilityId], any: &[CapabilityId]) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BuildRequest, Effect, VariableIdentity};

    #[test]
    fn compatibility_workspace_catalog_covers_every_screen_and_named_destination() {
        assert_eq!(WorkspaceDestination::ALL.len(), 24);
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
            Screen::Settings,
        ] {
            let _ = workspace_screen_destination(screen);
        }
        assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::Devtool));
        assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::QemuWic));
        assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::ProjectProfiles));
        assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::TerminalSessions));
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
}
