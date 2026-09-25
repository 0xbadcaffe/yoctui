use super::*;

fn install_utility_authority(app: &mut App, capabilities: &[CapabilityId]) {
    let implementations = capabilities
        .iter()
        .map(|capability| {
            (
                *capability,
                CapabilityImplementation {
                    id: format!("{}.argv", capability.as_str()),
                    kind: CapabilityImplementationKind::Command,
                },
            )
        })
        .collect();
    let snapshot = DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation: 7,
            environment: YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    "/work/build".into(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: AuthoritativeValue::detected(
                    vec![
                        ToolIdentity {
                            id: "bitbake-config-build".into(),
                            executable: "/work/bitbake/bin/bitbake-config-build".into(),
                            version: None,
                        },
                        ToolIdentity {
                            id: "bitbake-layers".into(),
                            executable: "/work/bitbake/bin/bitbake-layers".into(),
                            version: None,
                        },
                    ],
                    IdentityAuthority::ExecutableProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            },
            capabilities: capabilities
                .iter()
                .map(|capability| CapabilityRecord {
                    id: *capability,
                    state: CapabilityState::Available,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: CapabilityEvidenceOutcome::Positive,
                        subject: capability.as_str().into(),
                        detail: "available in fixture".into(),
                        argv: Vec::new(),
                    }],
                })
                .collect(),
        },
        implementations,
    };
    install_workspace_compatibility(app, snapshot).unwrap();
}

#[test]
fn yocto_utility_forms_build_exact_operation_specific_arguments() {
    let mut config = YoctoUtilityDialog::new(YoctoUtilityCommand::ConfigBuild);
    config.cycle_choice(2);
    config.select_field(1);
    for character in "machine/qemuarm64 core/yocto/sstate-mirror-cdn".chars() {
        config.append(character);
    }
    assert_eq!(
        config.arguments().unwrap(),
        [
            "enable-fragment",
            "machine/qemuarm64",
            "core/yocto/sstate-mirror-cdn"
        ]
    );

    let recipes = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersShowRecipes);
    assert_eq!(recipes.arguments().unwrap(), ["show-recipes", "linux-*"]);

    let mut create = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersCreateLayer);
    for character in "/layers/meta-demo".chars() {
        create.append(character);
    }
    create.select_field(1);
    create.cycle_choice(1);
    assert_eq!(
        create.arguments().unwrap(),
        ["create-layer", "--add-layer", "/layers/meta-demo"]
    );
}

#[test]
fn yocto_utility_review_uses_current_tool_and_build_authority() {
    let mut app = App::new(16, 4096);
    app.workspace.build_dir = Some("/work/build".into());
    install_utility_authority(
        &mut app,
        &[
            CapabilityId::BitBakeLayersShowRecipes,
            CapabilityId::BitBakeConfigBuildListFragments,
        ],
    );
    let _ = update(
        &mut app,
        Action::OpenYoctoUtility(YoctoUtilityCommand::LayersShowRecipes),
    );
    let _ = update(&mut app, Action::ReviewYoctoUtility);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request: TerminalLaunchRequest { program, cwd, arguments, .. },
            ..
        })) if program == std::path::Path::new("/work/bitbake/bin/bitbake-layers")
            && cwd == std::path::Path::new("/work/build")
            && arguments == &["show-recipes", "linux-*"]
    ));
}

#[test]
fn yocto_utility_validation_keeps_the_form_open_without_spawning() {
    let mut app = App::new(16, 4096);
    let _ = update(
        &mut app,
        Action::OpenYoctoUtility(YoctoUtilityCommand::LayersCreateLayer),
    );
    assert_eq!(update(&mut app, Action::ReviewYoctoUtility), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::YoctoUtility(YoctoUtilityDialog {
            validation_error: Some(_),
            ..
        }))
    ));
}
