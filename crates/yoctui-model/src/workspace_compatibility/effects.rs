pub fn workspace_effect_requirement(effect: &Effect) -> WorkspaceEffectRequirement {
    use CapabilityId as Id;
    use WorkspaceEffectRequirement as Requirement;

    match effect {
        Effect::PersistSettings
        | Effect::ReadEnvironmentDirectory { .. }
        | Effect::PersistOnboarding
        | Effect::GenerateProjectProfile { .. }
        | Effect::VerifyBuildEnvironment { .. }
        | Effect::CloneBuildEnvironment(_)
        | Effect::OpenInEditor(_)
        | Effect::CopyToClipboard(_)
        | Effect::Terminal(_)
        | Effect::LaunchDetachedTerminal(_)
        | Effect::OpenWorkspaceEditor { .. }
        | Effect::LoadLayerBrowserDirectory { .. }
        | Effect::LoadLayerBrowserPreview(_)
        | Effect::OpenLayerBrowserEditor { .. }
        | Effect::GetImageArtifacts(_)
        | Effect::CancelImageArtifactOperation
        | Effect::GetRootfsComposition(_)
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
        Effect::InspectKernel | Effect::InspectFirmware => {
            Requirement::all(&[Id::BitBakeGetVar, Id::BitBakeRecipeMetadata])
        }
        Effect::Cancel => Requirement::one(Id::BitBakeCancellation),
        Effect::StartRaw(request) => builtin_raw_catalog()
            .command(&request.command)
            .and_then(|command| match &command.execution {
                RawExecutionPolicy::Executable { template } => Some(match &template.capabilities {
                    RawCapabilityRequirement::All { capabilities } => Requirement::Capabilities {
                        all: capabilities.clone(),
                        any: Vec::new(),
                    },
                    RawCapabilityRequirement::Any { capabilities } => Requirement::Capabilities {
                        all: Vec::new(),
                        any: capabilities.clone(),
                    },
                }),
                RawExecutionPolicy::ReferenceOnly { .. } => None,
            })
            .unwrap_or_else(|| Requirement::Capabilities {
                all: vec![Id::BitBakeRawServerToken],
                any: vec![Id::BitBakeRawServerToken],
            }),
        Effect::CancelRaw(_) | Effect::SetRawAttachment { .. } => Requirement::ClientLocal,
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

