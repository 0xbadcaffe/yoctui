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
        WorkspaceDestination::Kernel | WorkspaceDestination::Firmware => {
            WorkspaceEffectRequirement::all_and_any(
                &[Id::BitBakeGetVar],
                &[Id::MenuConfig, Id::BitBakeRecipeInventory],
            )
        }
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
        WorkspaceDestination::RawMode => WorkspaceEffectRequirement::one(Id::BitBakeRawCli),
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

