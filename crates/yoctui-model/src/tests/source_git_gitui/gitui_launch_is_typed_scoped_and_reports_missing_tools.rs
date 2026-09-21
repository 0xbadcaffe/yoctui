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
