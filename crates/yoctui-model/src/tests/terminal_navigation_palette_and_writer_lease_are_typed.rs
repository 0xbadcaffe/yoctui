//! Regression tests grouped around ux_terminal_navigation_palette_and_writer_lease_are_typed.
use super::*;

#[test]
fn ux_terminal_navigation_palette_and_writer_lease_are_typed() {
    let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, Some([8; 16]));
    assert_eq!(
        command_action(&app, CommandId::OpenTerminalSessions),
        Action::Open(Screen::TerminalSessions)
    );
    assert!(
        app.command_palette_commands()
            .iter()
            .any(|command| { command.id == CommandId::OpenTerminalSessions && command.enabled() })
    );

    assert_eq!(update(&mut app, Action::TerminalTakeControl), None);
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("read-only"))
    );

    app.daemon.pty_details[0].writer = None;
    assert_eq!(
        update(&mut app, Action::TerminalTakeControl),
        Some(Effect::Terminal(TerminalEffect::TakeControl {
            session_id: 41,
            expected_epoch: 9,
        }))
    );
    app.daemon.pty_details[0].writer = Some([7; 16]);
    assert_eq!(
        update(&mut app, Action::TerminalReleaseControl),
        Some(Effect::Terminal(TerminalEffect::ReleaseControl {
            session_id: 41,
            writer_epoch: 9,
        }))
    );

    app.workspace.build_dir = Some("/work/build".into());
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        version: None,
        layer: None,
        preferred_version: None,
        file: Some("/layers/busybox.bb".into()),
        append_count: None,
    });
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(vec!["do_devshell".into(), "do_menuconfig".into()]),
            ..RecipeMetadata::default()
        },
    );
    assert_eq!(
        update(&mut app, Action::TerminalCreateSelectedDevshell),
        None
    );
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog {
            destination: TerminalLaunchDestination::Embedded,
            ..
        }))
    ));
    assert_eq!(
        update(&mut app, Action::ConfirmTerminalLaunch),
        Some(Effect::Terminal(TerminalEffect::Create {
            name: "devshell:busybox".into(),
            kind: TerminalCreationKind::Devshell,
            cwd: "/work/build".into(),
            program: "/usr/bin/env".into(),
            arguments: vec![
                "bitbake".into(),
                "busybox".into(),
                "-c".into(),
                "devshell".into(),
            ],
        }))
    );
}

#[test]
fn ux_terminal_paste_copy_scrollback_and_kill_are_explicit_and_bounded() {
    let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, Some([7; 16]));
    assert_eq!(
        update(
            &mut app,
            Action::TerminalStagePaste("echo reviewed\n".into())
        ),
        None
    );
    assert_eq!(app.terminal.mode, TerminalWorkbenchMode::PasteReview);
    assert_eq!(
        update(&mut app, Action::TerminalConfirmPaste),
        Some(Effect::Terminal(TerminalEffect::Input {
            session_id: 41,
            writer_epoch: 9,
            bytes: b"echo reviewed\n".to_vec(),
        }))
    );

    assert_eq!(
        update(&mut app, Action::TerminalScroll { delta: 500 }),
        Some(Effect::Terminal(TerminalEffect::Viewport {
            session_id: 41,
            scrollback_offset: 120,
        }))
    );
    assert_eq!(update(&mut app, Action::TerminalBeginKill), None);
    assert_eq!(app.terminal.mode, TerminalWorkbenchMode::KillConfirmation);
    assert_eq!(
        update(&mut app, Action::TerminalConfirmKill),
        Some(Effect::Terminal(TerminalEffect::Terminate {
            session_id: 41
        }))
    );

    app.daemon.pty_sessions[0].lifecycle = ClientDaemonLifecycle::Exited;
    assert_eq!(
        update(&mut app, Action::TerminalBeginKill),
        Some(Effect::Terminal(TerminalEffect::Close { session_id: 41 }))
    );
}

#[test]
fn devwork_terminal_chooser_is_zero_spawn_defaults_embedded_and_gates_detached() {
    let mut app = App::new(10, 1_000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/work/build".into());
    assert_eq!(update(&mut app, Action::TerminalCreateBuildShell), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog {
            destination: TerminalLaunchDestination::Embedded,
            ..
        }))
    ));
    let _ = update(
        &mut app,
        Action::SelectTerminalLaunchDestination { delta: 1 },
    );
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog {
            destination: TerminalLaunchDestination::Embedded,
            ..
        }))
    ));
    assert_eq!(update(&mut app, Action::CancelTerminalLaunch), None);
    assert!(app.active_dialog().is_none());

    let _ = update(
        &mut app,
        Action::DetachedTerminalAvailabilityDetected(DetachedTerminalAvailability::Available {
            launcher: "xterm".into(),
        }),
    );
    let _ = update(&mut app, Action::TerminalCreateBuildShell);
    let _ = update(
        &mut app,
        Action::SelectTerminalLaunchDestination { delta: 1 },
    );
    assert!(matches!(
        update(&mut app, Action::ConfirmTerminalLaunch),
        Some(Effect::LaunchDetachedTerminal(TerminalLaunchRequest {
            kind: TerminalCreationKind::BuildShell,
            ..
        }))
    ));
    assert!(app.active_dialog().is_none());
}

#[test]
fn platform_menuconfig_stays_in_its_workspace_until_the_pty_screen_is_ready() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Kernel;
    app.focus = FocusTarget::Workspace;
    app.daemon.status = ClientReplicaStatus::Current;
    app.terminal.client_id = Some([7; 16]);
    app.dialogs
        .push_back(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request: TerminalLaunchRequest {
                name: "kernel menuconfig".into(),
                kind: TerminalCreationKind::Menuconfig,
                cwd: "/work/build".into(),
                program: "/usr/bin/env".into(),
                arguments: vec!["/opt/bitbake/bin/bitbake".into()],
            },
            destination: TerminalLaunchDestination::Embedded,
            output_must_not_exist: None,
        }));

    assert!(matches!(
        update(&mut app, Action::ConfirmTerminalLaunch),
        Some(Effect::Terminal(TerminalEffect::Create {
            kind: TerminalCreationKind::Menuconfig,
            ..
        }))
    ));
    assert_eq!(app.screen, Screen::Kernel);
    assert!(app.platform_menuconfig_waiting());
    assert_eq!(
        app.transient_status(),
        Some(TransientStatus {
            kind: TransientStatusKind::Activity,
            text: "Starting Kernel menuconfig".into(),
        })
    );

    app.daemon.pty_sessions.push(ClientDaemonPtySummary {
        id: 52,
        name: "kernel menuconfig".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        viewers: 1,
    });
    app.daemon.pty_details.push(ClientDaemonPtyDetails {
        id: 52,
        kind: ClientDaemonPtyKind::Menuconfig,
        cwd: "/work/build".into(),
        columns: 120,
        rows: 40,
        writer: None,
        writer_epoch: 0,
        exit_code: None,
        restartable: true,
    });
    app.reconcile_platform_menuconfigs();
    assert!(app.platform_menuconfig_visible());
    assert_eq!(
        app.selected_terminal_session().map(|session| session.id),
        Some(52)
    );
    assert_eq!(
        app.pending_platform_writer_effect(),
        Some(TerminalEffect::TakeControl {
            session_id: 52,
            expected_epoch: 0,
        })
    );
    assert!(app.platform_menuconfig_waiting());

    app.daemon.pty_screens.push(ClientDaemonPtyScreen {
        session_id: 52,
        columns: 120,
        rows_count: 40,
        cursor_column: 0,
        cursor_row: 0,
        cursor_hidden: false,
        scrollback_offset: 0,
        rows: vec!["Linux Kernel Configuration".into()],
        cells: Vec::new(),
        scrollback_lines: 0,
        dropped_line_feeds_lower_bound: 0,
    });
    assert!(!app.platform_menuconfig_waiting());
}

#[test]
fn firmware_menuconfig_stays_in_the_u_boot_workspace() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Firmware;
    app.dialogs
        .push_back(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request: TerminalLaunchRequest {
                name: "u-boot menuconfig".into(),
                kind: TerminalCreationKind::Menuconfig,
                cwd: "/work/build".into(),
                program: "/usr/bin/env".into(),
                arguments: vec!["/opt/bitbake/bin/bitbake".into()],
            },
            destination: TerminalLaunchDestination::Embedded,
            output_must_not_exist: None,
        }));

    assert!(matches!(
        update(&mut app, Action::ConfirmTerminalLaunch),
        Some(Effect::Terminal(TerminalEffect::Create {
            kind: TerminalCreationKind::Menuconfig,
            ..
        }))
    ));
    assert_eq!(app.screen, Screen::Firmware);
    assert_eq!(
        app.firmware.menuconfig_terminal.name.as_deref(),
        Some("u-boot menuconfig")
    );
    assert_eq!(
        app.platform_menuconfig_waiting_label().as_deref(),
        Some("Starting U-Boot menuconfig")
    );
}

#[test]
fn devwork_terminal_devtool_routes_use_authoritative_recipe_and_workspace() {
    let mut app = App::new(10, 1_000);
    app.gitui_program = Some("/usr/bin/gitui".into());
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
    };
    app.workspace.build_dir = Some("/work/build".into());
    app.workspace.recipes.push(Recipe {
        name: identity.name.clone(),
        file: Some(identity.file.clone()),
        ..Recipe::default()
    });
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity: identity.clone(),
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: "/work/build/workspace/sources/busybox".into(),
                recipe_file: Some(identity.file.clone()),
            },
            git: DevtoolGitState::Available {
                branch: Some("devtool".into()),
                head: Some("abc123".into()),
                modified: 0,
                untracked: 0,
                conflicted: 0,
            },
            error: None,
        },
    );
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipeDevtoolWorkspaceShell),
        None
    );
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog { request, .. }))
            if request.kind == TerminalCreationKind::DevtoolShell
                && request.cwd.as_path() == Path::new("/work/build/workspace/sources/busybox")
                && request.program.as_path() == Path::new("/bin/sh")
    ));
    let _ = update(&mut app, Action::CancelTerminalLaunch);
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolGitUi);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog { request, .. }))
            if request.kind == TerminalCreationKind::GitUi
                && request.cwd.as_path() == Path::new("/work/build/workspace/sources/busybox")
                && request.program.as_path() == Path::new("/usr/bin/gitui")
                && request.arguments.is_empty()
    ));
    let _ = update(&mut app, Action::CancelTerminalLaunch);
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolEditRecipe);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog { request, .. }))
            if request.kind == TerminalCreationKind::Utility
                && request.arguments == vec!["devtool", "edit-recipe", "busybox"]
    ));
}

#[test]
fn build_authority_loss_retires_every_nonterminal_task_without_faking_failure() {
    let mut app = App::new(10, 1_000);
    app.build.status = BuildStatus::Parsing;
    app.build.target = Some("core-image-minimal".into());
    app.build.total = Some(400);
    app.build.completed = 28;
    let active = TaskInfo::active(
        TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    let mut queued = TaskInfo::active(
        TaskId("systemd:do_package".into()),
        "systemd".into(),
        "do_package".into(),
    );
    queued.state = TaskState::Queued;
    app.tasks.insert(active.id.clone(), active);
    app.tasks.insert(queued.id.clone(), queued);

    assert_eq!(app.waiting_task_count(), 370);
    assert_eq!(
        update(
            &mut app,
            Action::BuildAuthorityLost {
                message: "daemon socket closed".into(),
            },
        ),
        None
    );

    assert_eq!(app.build.status, BuildStatus::Lost);
    assert!(app.tasks.is_empty());
    assert_eq!(app.waiting_task_count(), 0);
    assert_eq!(app.completed_tasks.len(), 2);
    assert!(
        app.completed_tasks
            .iter()
            .all(|task| task.task.state == TaskState::Lost && !task.success)
    );
    assert_eq!(
        app.build.errors, 0,
        "transport loss is not a BitBake failure"
    );
    assert!(app.build_history.is_empty());
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("Reconnecting to the daemon"))
    );
}
