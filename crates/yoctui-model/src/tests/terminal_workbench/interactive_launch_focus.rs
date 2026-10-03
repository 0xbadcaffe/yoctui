use crate::tests::ux_terminal_fixture;
use crate::*;

fn request(kind: TerminalCreationKind) -> TerminalLaunchRequest {
    TerminalLaunchRequest {
        name: "interactive demo".into(),
        kind,
        cwd: "/work/build".into(),
        program: "/usr/bin/env".into(),
        arguments: if kind == TerminalCreationKind::Utility {
            vec!["devtool".into(), "edit-recipe".into(), "busybox".into()]
        } else {
            vec![]
        },
        completion: None,
    }
}

fn fixture() -> App {
    let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, Some([7; 16]));
    app.screen = Screen::Recipes;
    app.focus = FocusTarget::Navigator;
    app
}

#[test]
fn interactive_launch_focus_embedded_waits_for_new_slot_not_old_history() {
    for kind in [
        TerminalCreationKind::BuildShell,
        TerminalCreationKind::Devshell,
        TerminalCreationKind::Menuconfig,
        TerminalCreationKind::DevtoolShell,
        TerminalCreationKind::Utility,
    ] {
        for split in [false, true] {
            let mut app = fixture();
            if split {
                app.split_terminal_pane(SplitAxis::Horizontal).unwrap();
            }
            crate::recipe_operations::open_terminal_launch(&mut app, request(kind));
            assert_eq!(app.screen, Screen::Recipes);
            assert!(matches!(
                update(&mut app, Action::ConfirmTerminalLaunch),
                Some(Effect::Terminal(TerminalEffect::Create { kind: actual, .. }))
                    if actual == kind
            ));
            assert_eq!(app.screen, Screen::TerminalSessions, "{kind:?}");
            assert_eq!(app.focus, FocusTarget::Workspace);
            assert_eq!(app.focus_return, None);
            assert_eq!(app.selected_terminal_session(), None);
            assert!(!app.selected_terminal_is_writer());
            // No response/failure cannot redirect input to an existing writer.
            assert_eq!(update(&mut app, Action::TerminalTakeControl), None);
            let mut created = app.daemon.pty_sessions[0].clone();
            created.id = 42;
            app.daemon.pty_sessions.push(created);
            app.reconcile_terminal_panes();
            assert_eq!(app.selected_terminal_session().unwrap().id, 42);
            assert!(!app.selected_terminal_is_writer());
            assert_eq!(app.daemon.pty_sessions[0].id, 41);
        }
    }
}

#[test]
fn interactive_launch_focus_empty_history_keeps_pending_selection_bounded() {
    let mut app = fixture();
    app.daemon.pty_sessions.clear();
    crate::recipe_operations::open_terminal_launch(
        &mut app,
        request(TerminalCreationKind::BuildShell),
    );
    update(&mut app, Action::ConfirmTerminalLaunch);
    assert_eq!(app.screen, Screen::TerminalSessions);
    assert_eq!(app.pty_selection, 0);
    assert_eq!(app.selected_terminal_session(), None);
    assert!(!app.selected_terminal_is_writer());
}

#[test]
fn interactive_launch_focus_cancel_and_detached_preserve_view_and_binding() {
    for detached in [false, true] {
        let mut app = fixture();
        app.split_terminal_pane(SplitAxis::Vertical).unwrap();
        let before = (
            app.screen,
            app.focus,
            app.pty_selection,
            app.terminal.clone(),
        );
        crate::recipe_operations::open_terminal_launch(
            &mut app,
            request(TerminalCreationKind::Devshell),
        );
        if detached {
            let Some(Dialog::TerminalLaunch(dialog)) = app.active_dialog_mut() else {
                panic!("missing chooser");
            };
            dialog.destination = TerminalLaunchDestination::Detached;
            assert!(matches!(
                update(&mut app, Action::ConfirmTerminalLaunch),
                Some(Effect::LaunchDetachedTerminal(_))
            ));
        } else {
            assert_eq!(update(&mut app, Action::CancelTerminalLaunch), None);
        }
        assert_eq!(
            (app.screen, app.focus, app.pty_selection, app.terminal),
            before
        );
    }
}

#[test]
fn interactive_launch_focus_platform_and_completion_keep_specialized_routes() {
    for screen in [Screen::Kernel, Screen::Firmware] {
        let mut app = fixture();
        app.screen = screen;
        crate::recipe_operations::open_terminal_launch(
            &mut app,
            request(TerminalCreationKind::Menuconfig),
        );
        update(&mut app, Action::ConfirmTerminalLaunch);
        assert_eq!(app.screen, screen);
        assert_eq!(app.focus, FocusTarget::Workspace);
        assert!(app.platform_menuconfig_pending());
    }
    let mut app = fixture();
    let before = (app.screen, app.pty_selection, app.terminal.clone());
    let mut request = request(TerminalCreationKind::Utility);
    request.completion = Some(TerminalCompletion::OpenDeviceTree {
        component: PlatformComponent::Kernel,
        path: "/work/output.dts".into(),
    });
    crate::recipe_operations::open_terminal_launch(&mut app, request);
    update(&mut app, Action::ConfirmTerminalLaunch);
    assert_eq!((app.screen, app.pty_selection, app.terminal), before);
}

#[test]
fn interactive_launch_focus_unrelated_utility_does_not_retarget_selection() {
    let mut app = fixture();
    let before = (app.screen, app.pty_selection, app.terminal.clone());
    let mut request = request(TerminalCreationKind::Utility);
    request.arguments = vec!["devtool".into(), "status".into()];
    crate::recipe_operations::open_terminal_launch(&mut app, request);
    assert!(matches!(
        update(&mut app, Action::ConfirmTerminalLaunch),
        Some(Effect::Terminal(TerminalEffect::Create { .. }))
    ));
    assert_eq!((app.screen, app.pty_selection, app.terminal), before);
}
