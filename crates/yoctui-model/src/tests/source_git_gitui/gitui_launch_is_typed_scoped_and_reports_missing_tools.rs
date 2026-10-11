use super::*;

#[test]
fn gitui_launch_is_typed_scoped_and_reports_missing_tools() {
    let mut app = App::new(10, 1024);
    update(&mut app, Action::OpenGitUi);
    assert!(app.notification.as_ref().unwrap().contains("not installed"));
    update(
        &mut app,
        Action::GitUiDetected(Some("/usr/bin/gitui".into())),
    );
    app.workspace.source_dir = Some("/source with spaces".into());
    app.source_git_status = SourceGitStatus::Ready(SourceGitSummary::default());
    let command = app
        .application_menu_items(ApplicationMenuGroup::Tools)
        .into_iter()
        .find(|c| c.label == "Open GitUI")
        .unwrap();
    assert!(command.enabled());
    update(&mut app, Action::OpenGitUi);
    let Some(Dialog::TerminalLaunch(dialog)) = app.active_dialog() else {
        panic!("missing GitUI launch dialog");
    };
    assert_eq!(
        dialog.request.cwd,
        std::path::PathBuf::from("/source with spaces")
    );
    assert_eq!(dialog.request.kind, TerminalCreationKind::GitUi);
    assert!(dialog.request.arguments.is_empty());
    assert!(matches!(
        update(&mut app, Action::ConfirmTerminalLaunch),
        Some(Effect::Terminal(TerminalEffect::Create {
            kind: TerminalCreationKind::GitUi,
            ..
        }))
    ));
    assert_eq!(app.screen, Screen::TerminalSessions);
    assert_eq!(app.focus, FocusTarget::Workspace);
}

#[test]
fn navigator_gitui_reuses_menu_launch_and_embedded_confirmation() {
    let mut app = App::new(10, 1024);
    app.onboarding.open = false;
    app.gitui_program = Some("/usr/bin/gitui".into());
    app.workspace.source_dir = Some("/source with spaces".into());
    app.source_git_status = SourceGitStatus::Ready(SourceGitSummary::default());
    app.navigator_selection = App::navigator_selection_for_destination(WorkspaceDestination::GitUi)
        .expect("GitUI must have a navigator entry");
    assert_eq!(NAVIGATOR_GROUPS[app.navigator_group_index()].label, "TOOLS");
    let mut from_menu = app.clone();
    update(&mut from_menu, Action::OpenGitUi);
    update(&mut app, Action::ActivateNavigator);
    assert_eq!(app.dialogs, from_menu.dialogs);
    assert_eq!(app.notification, from_menu.notification);
    let Some(Dialog::TerminalLaunch(dialog)) = app.active_dialog() else {
        panic!("navigator must use the existing GitUI launch chooser");
    };
    assert_eq!(dialog.destination, TerminalLaunchDestination::Embedded);
    assert_eq!(
        dialog.request.cwd,
        std::path::PathBuf::from("/source with spaces")
    );
    assert!(app.daemon.pty_sessions.is_empty());
    assert!(matches!(
        update(&mut app, Action::ConfirmTerminalLaunch),
        Some(Effect::Terminal(TerminalEffect::Create {
            kind: TerminalCreationKind::GitUi,
            ..
        }))
    ));
    assert_eq!(app.screen, Screen::TerminalSessions);
    assert_eq!(app.focus, FocusTarget::Workspace);
}

#[test]
fn navigator_gitui_preserves_missing_tool_and_repository_checks() {
    for (program, source, status) in [
        (None, None, SourceGitStatus::Unknown),
        (
            Some("/usr/bin/gitui".into()),
            None,
            SourceGitStatus::Unknown,
        ),
        (
            Some("/usr/bin/gitui".into()),
            Some("/source".into()),
            SourceGitStatus::Scanning,
        ),
    ] {
        let mut app = App::new(10, 1024);
        app.onboarding.open = false;
        app.gitui_program = program;
        app.workspace.source_dir = source;
        app.source_git_status = status;
        app.navigator_selection =
            App::navigator_selection_for_destination(WorkspaceDestination::GitUi).unwrap();
        let mut from_menu = app.clone();
        update(&mut from_menu, Action::OpenGitUi);
        update(&mut app, Action::ActivateNavigator);
        assert_eq!(app.notification, from_menu.notification);
        assert!(app.notification.is_some());
        assert!(app.active_dialog().is_none());
        assert_eq!(app.screen, from_menu.screen);
        assert_eq!(app.focus, from_menu.focus);
    }
}
