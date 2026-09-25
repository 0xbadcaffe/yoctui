use super::*;

fn choose(dialog: &mut YoctoUtilityDialog, field: usize, expected: &str) {
    dialog.selected_field = field;
    for _ in 0..8 {
        if dialog.fields()[field].1 == expected {
            return;
        }
        dialog.cycle_choice(1);
    }
    panic!("choice {expected:?} was not available for field {field}");
}

fn valid_dialog(command: DevtoolUtilityCommand) -> YoctoUtilityDialog {
    let mut dialog = YoctoUtilityDialog::new(YoctoUtilityCommand::Devtool(command));
    match command {
        DevtoolUtilityCommand::Add
        | DevtoolUtilityCommand::Status
        | DevtoolUtilityCommand::CheckUpgradeStatus
        | DevtoolUtilityCommand::BuildImage
        | DevtoolUtilityCommand::CreateWorkspace
        | DevtoolUtilityCommand::Export => {}
        DevtoolUtilityCommand::Modify
        | DevtoolUtilityCommand::Upgrade
        | DevtoolUtilityCommand::LatestVersion
        | DevtoolUtilityCommand::Build
        | DevtoolUtilityCommand::Rename
        | DevtoolUtilityCommand::EditRecipe
        | DevtoolUtilityCommand::FindRecipe
        | DevtoolUtilityCommand::ConfigureHelp
        | DevtoolUtilityCommand::UpdateRecipe
        | DevtoolUtilityCommand::Reset
        | DevtoolUtilityCommand::Menuconfig => set_text(&mut dialog, 0, "busybox"),
        DevtoolUtilityCommand::Search => set_text(&mut dialog, 0, "network.*tool"),
        DevtoolUtilityCommand::IdeSdk => set_text(&mut dialog, 0, "busybox core-image-minimal"),
        DevtoolUtilityCommand::Finish => {
            set_text(&mut dialog, 0, "busybox");
            set_text(&mut dialog, 1, "meta-local");
        }
        DevtoolUtilityCommand::DeployTarget | DevtoolUtilityCommand::UndeployTarget => {
            set_text(&mut dialog, 0, "busybox");
            set_text(&mut dialog, 1, "root@192.0.2.1");
        }
        DevtoolUtilityCommand::Extract | DevtoolUtilityCommand::Sync => {
            set_text(&mut dialog, 0, "busybox");
            set_text(&mut dialog, 1, "/work/sources/busybox");
        }
        DevtoolUtilityCommand::Import => set_text(&mut dialog, 0, "/work/export.tar.gz"),
    }
    dialog
}

#[test]
fn every_devtool_utility_subcommand_builds_exact_minimal_argv() {
    let expected = [
        (DevtoolUtilityCommand::Add, vec!["add"]),
        (DevtoolUtilityCommand::Modify, vec!["modify", "busybox"]),
        (DevtoolUtilityCommand::Upgrade, vec!["upgrade", "busybox"]),
        (DevtoolUtilityCommand::Status, vec!["status"]),
        (
            DevtoolUtilityCommand::LatestVersion,
            vec!["latest-version", "busybox"],
        ),
        (
            DevtoolUtilityCommand::CheckUpgradeStatus,
            vec!["check-upgrade-status"],
        ),
        (
            DevtoolUtilityCommand::Search,
            vec!["search", "network.*tool"],
        ),
        (DevtoolUtilityCommand::Build, vec!["build", "busybox"]),
        (
            DevtoolUtilityCommand::IdeSdk,
            vec!["ide-sdk", "busybox", "core-image-minimal"],
        ),
        (DevtoolUtilityCommand::Rename, vec!["rename", "busybox"]),
        (
            DevtoolUtilityCommand::EditRecipe,
            vec!["edit-recipe", "busybox"],
        ),
        (
            DevtoolUtilityCommand::FindRecipe,
            vec!["find-recipe", "busybox"],
        ),
        (
            DevtoolUtilityCommand::ConfigureHelp,
            vec!["configure-help", "busybox"],
        ),
        (
            DevtoolUtilityCommand::UpdateRecipe,
            vec!["update-recipe", "busybox"],
        ),
        (DevtoolUtilityCommand::Reset, vec!["reset", "busybox"]),
        (
            DevtoolUtilityCommand::Finish,
            vec!["finish", "busybox", "meta-local"],
        ),
        (
            DevtoolUtilityCommand::DeployTarget,
            vec!["deploy-target", "busybox", "root@192.0.2.1"],
        ),
        (
            DevtoolUtilityCommand::UndeployTarget,
            vec!["undeploy-target", "busybox", "root@192.0.2.1"],
        ),
        (DevtoolUtilityCommand::BuildImage, vec!["build-image"]),
        (
            DevtoolUtilityCommand::CreateWorkspace,
            vec!["create-workspace"],
        ),
        (DevtoolUtilityCommand::Export, vec!["export"]),
        (
            DevtoolUtilityCommand::Extract,
            vec!["extract", "busybox", "/work/sources/busybox"],
        ),
        (
            DevtoolUtilityCommand::Sync,
            vec!["sync", "busybox", "/work/sources/busybox"],
        ),
        (
            DevtoolUtilityCommand::Import,
            vec!["import", "/work/export.tar.gz"],
        ),
        (
            DevtoolUtilityCommand::Menuconfig,
            vec!["menuconfig", "busybox"],
        ),
    ];
    assert_eq!(expected.len(), DevtoolUtilityCommand::ALL.len());
    for (command, expected) in expected {
        assert_eq!(
            valid_dialog(command).arguments().unwrap(),
            expected,
            "wrong argv for {}",
            command.subcommand()
        );
    }
}

#[test]
fn devtool_add_and_ide_sdk_preserve_every_exposed_option() {
    let mut add = YoctoUtilityDialog::new(YoctoUtilityCommand::Devtool(DevtoolUtilityCommand::Add));
    set_text(&mut add, 0, "demo");
    set_text(&mut add, 1, "/work/demo");
    set_text(&mut add, 2, "git://example.invalid/demo");
    choose(&mut add, 3, "--fetch (deprecated)");
    choose(&mut add, 4, "same source");
    for field in [5, 6, 8, 12, 13, 15] {
        toggle(&mut add, field);
    }
    set_text(&mut add, 7, "1.2");
    choose(&mut add, 9, "fixed SRCREV");
    set_text(&mut add, 10, "deadbeef");
    set_text(&mut add, 11, "main");
    set_text(&mut add, 14, "src");
    set_text(&mut add, 16, "virtual/demo");
    assert_eq!(
        add.arguments().unwrap(),
        [
            "add",
            "--same-dir",
            "--npm-dev",
            "--no-pypi",
            "--version",
            "1.2",
            "--no-git",
            "--srcrev",
            "deadbeef",
            "--srcbranch",
            "main",
            "--binary",
            "--also-native",
            "--mirrors",
            "--src-subdir",
            "src",
            "--provides",
            "virtual/demo",
            "--fetch",
            "git://example.invalid/demo",
            "demo",
            "/work/demo"
        ]
    );

    let mut ide =
        YoctoUtilityDialog::new(YoctoUtilityCommand::Devtool(DevtoolUtilityCommand::IdeSdk));
    set_text(&mut ide, 0, "demo core-image-minimal");
    choose(&mut ide, 1, "shared");
    choose(&mut ide, 2, "none");
    set_text(&mut ide, 3, "root@target");
    set_text(&mut ide, 4, "2345");
    set_text(&mut ide, 6, "ssh");
    set_text(&mut ide, 7, "2222");
    set_text(&mut ide, 8, "/home/user/key");
    for field in [5, 9, 10, 11, 12, 13, 14, 15] {
        toggle(&mut ide, field);
    }
    assert_eq!(
        ide.arguments().unwrap(),
        [
            "ide-sdk",
            "--mode",
            "shared",
            "--ide",
            "none",
            "--target",
            "root@target",
            "--gdbserver-port-start",
            "2345",
            "--no-host-check",
            "--ssh-exec",
            "ssh",
            "--port",
            "2222",
            "--key",
            "/home/user/key",
            "--skip-bitbake",
            "--bitbake-k",
            "--no-strip",
            "--dry-run",
            "--show-status",
            "--no-preserve",
            "--no-check-space",
            "demo",
            "core-image-minimal"
        ]
    );
}

#[test]
fn devtool_mutating_forms_build_options_and_reject_ambiguous_requests() {
    let mut update = valid_dialog(DevtoolUtilityCommand::UpdateRecipe);
    choose(&mut update, 1, "patch");
    set_text(&mut update, 2, "base-rev");
    set_text(&mut update, 3, "/layers/meta-local");
    for field in 4..=8 {
        toggle(&mut update, field);
    }
    assert_eq!(
        update.arguments().unwrap(),
        [
            "update-recipe",
            "--mode",
            "patch",
            "--initial-rev",
            "base-rev",
            "--append",
            "/layers/meta-local",
            "--wildcard-version",
            "--no-remove",
            "--no-overrides",
            "--dry-run",
            "--force-patch-refresh",
            "busybox"
        ]
    );

    let mut reset = valid_dialog(DevtoolUtilityCommand::Reset);
    toggle(&mut reset, 1);
    assert_eq!(
        reset.arguments().unwrap_err(),
        "reset all cannot be combined with recipe names"
    );
    set_text(&mut reset, 0, "");
    assert_eq!(reset.arguments().unwrap(), ["reset", "--all"]);

    let mut undeploy = valid_dialog(DevtoolUtilityCommand::UndeployTarget);
    toggle(&mut undeploy, 4);
    assert_eq!(
        undeploy.arguments().unwrap_err(),
        "undeploy all cannot be combined with a recipe name"
    );
}

#[test]
fn every_devtool_menu_command_opens_its_typed_form() {
    let app = App::new(16, 4096);
    for command in DevtoolUtilityCommand::ALL {
        assert_eq!(
            command_action(&app, CommandId::OpenDevtool(command)),
            Action::OpenYoctoUtility(YoctoUtilityCommand::Devtool(command))
        );
    }
}

#[test]
fn application_menu_has_a_dedicated_complete_devtool_group() {
    let mut app = App::new(16, 4096);
    let _ = update(&mut app, Action::OpenApplicationMenu);
    for _ in 0..5 {
        let _ = update(&mut app, Action::SelectMenuGroup { delta: 1 });
    }
    assert_eq!(app.menu.group(), ApplicationMenuGroup::Devtool);
    let items = app.active_menu_items();
    assert_eq!(items.len(), DevtoolUtilityCommand::ALL.len());
    for command in DevtoolUtilityCommand::ALL {
        assert!(
            items.iter().any(|item| item.label == command.label()),
            "missing {}",
            command.subcommand()
        );
    }
}

#[test]
fn devtool_review_uses_initialized_executable_and_build_directory() {
    let mut app = App::new(16, 4096);
    app.workspace.build_dir = Some("/work/build".into());
    install_utility_authority(&mut app, &[CapabilityId::DevtoolStatus]);
    let _ = update(
        &mut app,
        Action::OpenYoctoUtility(YoctoUtilityCommand::Devtool(DevtoolUtilityCommand::Status)),
    );
    let _ = update(&mut app, Action::ReviewYoctoUtility);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request: TerminalLaunchRequest { program, cwd, arguments, .. },
            ..
        })) if program == std::path::Path::new("/work/bitbake/bin/devtool")
            && cwd == std::path::Path::new("/work/build")
            && arguments == &["status"]
    ));
}
