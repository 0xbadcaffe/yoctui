pub fn workspace_dialog_requirement(dialog: &Dialog) -> WorkspaceEffectRequirement {
    use CapabilityId as Id;
    match dialog {
        Dialog::BuildEnvironmentCloneEditor(_)
        | Dialog::EnvironmentSetup(_)
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
        | Dialog::RecipePicker(_)
        | Dialog::ConfigSourcePicker(_)
        | Dialog::ConfigScopePicker(_)
        | Dialog::ConfigComparison(_)
        | Dialog::ConfigEdit { .. }
        | Dialog::ConfigEditConfirmation(_)
        | Dialog::BbmaskEdit(_)
        | Dialog::BbmaskConfirmation(_)
        | Dialog::ImageConsole(_)
        | Dialog::DtcCompile(_)
        | Dialog::YoctoUtility(_)
        | Dialog::TerminalLaunch(_)
        | Dialog::RecipeEditor(_)
        | Dialog::BuildCancellationConfirmation
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
        Dialog::DevtoolUndeploy(_) | Dialog::DevtoolUndeployConfirmation(_) => {
            WorkspaceEffectRequirement::one(Id::DevtoolUndeployTarget)
        }
        Dialog::DevtoolUpgradeConfirmation(_) => {
            WorkspaceEffectRequirement::one(Id::DevtoolUpgrade)
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
