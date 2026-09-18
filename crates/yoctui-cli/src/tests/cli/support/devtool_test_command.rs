use super::*;

pub(crate) fn devtool_test_command(
    executable: PathBuf,
    operation: &DevtoolOperation,
) -> DevtoolCommandSpec {
    let (capability, implementation) = match operation {
        DevtoolOperation::Modify { .. } => (
            yoctui_model::CapabilityId::DevtoolModify,
            yoctui_bitbake::DEVTOOL_MODIFY_IMPLEMENTATION,
        ),
        DevtoolOperation::UpdateRecipe { .. } => (
            yoctui_model::CapabilityId::DevtoolUpdateRecipe,
            yoctui_bitbake::DEVTOOL_UPDATE_RECIPE_IMPLEMENTATION,
        ),
        DevtoolOperation::Finish { .. } => (
            yoctui_model::CapabilityId::DevtoolFinish,
            yoctui_bitbake::DEVTOOL_FINISH_IMPLEMENTATION,
        ),
        DevtoolOperation::DeployTarget { .. } => (
            yoctui_model::CapabilityId::DevtoolDeployTarget,
            yoctui_bitbake::DEVTOOL_DEPLOY_TARGET_IMPLEMENTATION,
        ),
        DevtoolOperation::UndeployTarget { .. } => (
            yoctui_model::CapabilityId::DevtoolUndeployTarget,
            yoctui_bitbake::DEVTOOL_UNDEPLOY_TARGET_IMPLEMENTATION,
        ),
        DevtoolOperation::Reset { .. } => (
            yoctui_model::CapabilityId::DevtoolReset,
            yoctui_bitbake::DEVTOOL_RESET_IMPLEMENTATION,
        ),
        DevtoolOperation::Upgrade { .. } => (
            yoctui_model::CapabilityId::DevtoolUpgrade,
            yoctui_bitbake::DEVTOOL_UPGRADE_IMPLEMENTATION,
        ),
    };
    let build_directory = std::env::temp_dir();
    let compatibility = yoctui_model::DaemonCompatibilitySnapshot {
        snapshot: yoctui_model::CapabilitySnapshot {
            generation: 1,
            environment: yoctui_model::YoctoEnvironmentIdentity {
                build_directory: yoctui_model::AuthoritativeValue::detected(
                    build_directory.clone(),
                    yoctui_model::IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: yoctui_model::AuthoritativeValue::detected(
                    vec![yoctui_model::ToolIdentity {
                        id: "devtool".into(),
                        executable: executable.clone(),
                        version: None,
                    }],
                    yoctui_model::IdentityAuthority::ExecutableProbe,
                ),
                ..yoctui_model::YoctoEnvironmentIdentity::default()
            },
            capabilities: vec![yoctui_model::CapabilityRecord {
                id: capability,
                state: yoctui_model::CapabilityState::Available,
                evidence: vec![yoctui_model::CapabilityEvidence {
                    kind: yoctui_model::CapabilityEvidenceKind::DirectProbe,
                    outcome: yoctui_model::CapabilityEvidenceOutcome::Positive,
                    subject: format!("{} test probe", capability.as_str()),
                    detail: "Fixture exposes this exact Devtool subcommand.".into(),
                    argv: vec![executable.display().to_string(), "--help".into()],
                }],
            }],
        },
        implementations: std::collections::BTreeMap::from([(
            capability,
            yoctui_model::CapabilityImplementation {
                id: implementation.into(),
                kind: yoctui_model::CapabilityImplementationKind::Command,
            },
        )]),
    }
    .normalize()
    .unwrap();
    DevtoolCommandSpec::with_executable(
        executable,
        operation,
        &compatibility,
        compatibility.snapshot.generation,
        &build_directory,
    )
    .unwrap()
}
