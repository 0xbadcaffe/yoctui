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

fn set_text(dialog: &mut YoctoUtilityDialog, field: usize, value: &str) {
    dialog.selected_field = field;
    dialog.clear();
    for character in value.chars() {
        dialog.append(character);
    }
}

fn toggle(dialog: &mut YoctoUtilityDialog, field: usize) {
    dialog.selected_field = field;
    dialog.cycle_choice(1);
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

    let mut recipes = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersShowRecipes);
    toggle(&mut recipes, 1);
    toggle(&mut recipes, 2);
    toggle(&mut recipes, 3);
    set_text(&mut recipes, 4, "kernel,module");
    set_text(&mut recipes, 5, "meta-core");
    toggle(&mut recipes, 6);
    toggle(&mut recipes, 7);
    set_text(&mut recipes, 8, "mc-a");
    assert_eq!(
        recipes.arguments().unwrap(),
        [
            "show-recipes",
            "-f",
            "-r",
            "-m",
            "-b",
            "--show-variants",
            "-i",
            "kernel,module",
            "-l",
            "meta-core",
            "--mc",
            "mc-a",
            "linux-*"
        ]
    );

    let mut create = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersCreateLayer);
    set_text(&mut create, 0, "/layers/meta-demo");
    toggle(&mut create, 1);
    set_text(&mut create, 2, "demo");
    set_text(&mut create, 3, "7");
    set_text(&mut create, 4, "example");
    set_text(&mut create, 5, "1.0");
    assert_eq!(
        create.arguments().unwrap(),
        [
            "create-layer",
            "--add-layer",
            "--layerid",
            "demo",
            "--priority",
            "7",
            "--example-recipe-name",
            "example",
            "--example-recipe-version",
            "1.0",
            "/layers/meta-demo"
        ]
    );
}

#[test]
fn every_bitbake_layers_form_builds_its_documented_argument_shape() {
    let no_fields = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersShowLayers);
    assert_eq!(no_fields.arguments().unwrap(), ["show-layers"]);

    let mut overlayed = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersShowOverlayed);
    toggle(&mut overlayed, 0);
    toggle(&mut overlayed, 1);
    set_text(&mut overlayed, 2, "mc-a");
    assert_eq!(
        overlayed.arguments().unwrap(),
        ["show-overlayed", "-f", "-s", "--mc", "mc-a"]
    );

    let mut appends = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersShowAppends);
    set_text(&mut appends, 0, "linux-* busybox");
    set_text(&mut appends, 1, "mc-b");
    assert_eq!(
        appends.arguments().unwrap(),
        ["show-appends", "--mc", "mc-b", "linux-*", "busybox"]
    );

    let mut cross = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersShowCrossDepends);
    toggle(&mut cross, 0);
    set_text(&mut cross, 1, "core,openembedded-layer");
    assert_eq!(
        cross.arguments().unwrap(),
        ["show-cross-depends", "-f", "-i", "core,openembedded-layer"]
    );

    let mut add = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersAddLayer);
    set_text(&mut add, 0, "/layers/meta-one /layers/meta-two");
    assert_eq!(
        add.arguments().unwrap(),
        ["add-layer", "/layers/meta-one", "/layers/meta-two"]
    );

    let mut remove = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersRemoveLayer);
    set_text(&mut remove, 0, "/layers/meta-old*");
    assert_eq!(
        remove.arguments().unwrap(),
        ["remove-layer", "/layers/meta-old*"]
    );

    let mut flatten = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersFlatten);
    set_text(&mut flatten, 0, "meta-core meta-openembedded");
    set_text(&mut flatten, 1, "/work/flattened");
    assert_eq!(
        flatten.arguments().unwrap(),
        [
            "flatten",
            "meta-core",
            "meta-openembedded",
            "/work/flattened"
        ]
    );

    let mut fetch = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersLayerIndexFetch);
    set_text(&mut fetch, 0, "meta-clang meta-rust");
    toggle(&mut fetch, 1);
    set_text(&mut fetch, 2, "master");
    toggle(&mut fetch, 3);
    set_text(&mut fetch, 4, "core");
    set_text(&mut fetch, 5, "/work/layers");
    assert_eq!(
        fetch.arguments().unwrap(),
        [
            "layerindex-fetch",
            "-n",
            "-s",
            "-b",
            "master",
            "-i",
            "core",
            "-f",
            "/work/layers",
            "meta-clang",
            "meta-rust"
        ]
    );

    let mut depends = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersLayerIndexShowDepends);
    set_text(&mut depends, 0, "meta-clang");
    set_text(&mut depends, 1, "master");
    assert_eq!(
        depends.arguments().unwrap(),
        ["layerindex-show-depends", "-b", "master", "meta-clang"]
    );

    let mut machines = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersShowMachines);
    toggle(&mut machines, 0);
    set_text(&mut machines, 1, "meta-aspeed");
    assert_eq!(
        machines.arguments().unwrap(),
        ["show-machines", "-b", "-l", "meta-aspeed"]
    );

    let mut save = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersSaveBuildConf);
    set_text(&mut save, 0, "/layers/meta-local");
    set_text(&mut save, 1, "production");
    assert_eq!(
        save.arguments().unwrap(),
        ["save-build-conf", "/layers/meta-local", "production"]
    );

    let mut setup = YoctoUtilityDialog::new(YoctoUtilityCommand::LayersCreateLayersSetup);
    set_text(&mut setup, 0, "/work/setup");
    set_text(&mut setup, 1, "romulus");
    set_text(&mut setup, 2, "oe-setup-layers");
    toggle(&mut setup, 3);
    toggle(&mut setup, 4);
    set_text(&mut setup, 5, "openbmc:master poky:nanbield");
    assert_eq!(
        setup.arguments().unwrap(),
        [
            "create-layers-setup",
            "--json-only",
            "--update",
            "--output-prefix",
            "romulus",
            "--writer",
            "oe-setup-layers",
            "--use-custom-reference",
            "openbmc:master",
            "--use-custom-reference",
            "poky:nanbield",
            "/work/setup"
        ]
    );
}

#[test]
fn every_bitbake_layers_menu_entry_opens_its_typed_form() {
    let app = App::new(16, 4096);
    let cases = [
        (
            CommandId::OpenBitBakeLayersShowLayers,
            YoctoUtilityCommand::LayersShowLayers,
        ),
        (
            CommandId::OpenBitBakeLayersShowRecipes,
            YoctoUtilityCommand::LayersShowRecipes,
        ),
        (
            CommandId::OpenBitBakeLayersShowOverlayed,
            YoctoUtilityCommand::LayersShowOverlayed,
        ),
        (
            CommandId::OpenBitBakeLayersShowAppends,
            YoctoUtilityCommand::LayersShowAppends,
        ),
        (
            CommandId::OpenBitBakeLayersShowCrossDepends,
            YoctoUtilityCommand::LayersShowCrossDepends,
        ),
        (
            CommandId::OpenBitBakeLayersAddLayer,
            YoctoUtilityCommand::LayersAddLayer,
        ),
        (
            CommandId::OpenBitBakeLayersRemoveLayer,
            YoctoUtilityCommand::LayersRemoveLayer,
        ),
        (
            CommandId::OpenBitBakeLayersFlatten,
            YoctoUtilityCommand::LayersFlatten,
        ),
        (
            CommandId::OpenBitBakeLayersLayerIndexFetch,
            YoctoUtilityCommand::LayersLayerIndexFetch,
        ),
        (
            CommandId::OpenBitBakeLayersLayerIndexShowDepends,
            YoctoUtilityCommand::LayersLayerIndexShowDepends,
        ),
        (
            CommandId::OpenBitBakeLayersCreateLayer,
            YoctoUtilityCommand::LayersCreateLayer,
        ),
        (
            CommandId::OpenBitBakeLayersShowMachines,
            YoctoUtilityCommand::LayersShowMachines,
        ),
        (
            CommandId::OpenBitBakeLayersSaveBuildConf,
            YoctoUtilityCommand::LayersSaveBuildConf,
        ),
        (
            CommandId::OpenBitBakeLayersCreateLayersSetup,
            YoctoUtilityCommand::LayersCreateLayersSetup,
        ),
    ];
    for (command, utility) in cases {
        assert_eq!(
            command_action(&app, command),
            Action::OpenYoctoUtility(utility)
        );
    }
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
