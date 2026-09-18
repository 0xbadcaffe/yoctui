use super::*;

#[tokio::test]
async fn client_devtool_status_uses_installed_daemon_compatibility_authority() {
    let build_directory = std::env::temp_dir();
    let executable = PathBuf::from("/bin/true");
    let capability = yoctui_model::CapabilityId::DevtoolStatus;
    let authority = yoctui_model::DaemonCompatibilitySnapshot {
        snapshot: yoctui_model::CapabilitySnapshot {
            generation: 7,
            environment: yoctui_model::YoctoEnvironmentIdentity {
                build_directory: yoctui_model::AuthoritativeValue::detected(
                    build_directory.clone(),
                    yoctui_model::IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: yoctui_model::AuthoritativeValue::detected(
                    vec![yoctui_model::ToolIdentity {
                        id: "devtool".into(),
                        executable,
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
                    subject: "devtool status test probe".into(),
                    detail: "Fixture exposes a successful status command.".into(),
                    argv: vec!["/bin/true".into(), "status".into()],
                }],
            }],
        },
        implementations: std::collections::BTreeMap::from([(
            capability,
            yoctui_model::CapabilityImplementation {
                id: yoctui_bitbake::DEVTOOL_STATUS_IMPLEMENTATION.into(),
                kind: yoctui_model::CapabilityImplementationKind::Command,
            },
        )]),
    }
    .normalize()
    .unwrap();
    let mut app = App::new(16, 4096);
    yoctui_model::install_workspace_compatibility(&mut app, authority).unwrap();
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
    };

    let status = inspect_devtool_status(&app, &build_directory, identity.clone()).await;

    assert_eq!(status.identity, identity);
    assert_eq!(
        status.capability,
        yoctui_model::DevtoolCapability::Available
    );
    assert_eq!(status.workspace, DevtoolWorkspace::NotMember);
    assert!(status.error.is_none());
}
