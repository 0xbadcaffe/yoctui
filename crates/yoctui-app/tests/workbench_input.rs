use yoctui_app::*;
use yoctui_model::*;
fn fixture() -> App {
    let mut app = App::new(32, 8192);
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::OpenRecipe {
            recipe: "busybox".into(),
            root: "/tmp/recipe".into(),
        }),
    );
    app
}
#[test]
fn workbench_editor_typing_does_not_trigger_global_shortcuts() {
    let mut app = fixture();
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::Focus(WorkbenchFocus::Editor)),
    );
    for c in ['q', 'b', 'i', '/', 't', 'n'] {
        assert_eq!(
            workbench_action(&app, Input::Char(c)),
            Some(Action::Workbench(WorkbenchAction::Edit(
                PopupEditorCommand::Insert(c)
            )))
        );
    }
}
#[test]
fn workbench_forms_trap_navigation_and_paste_is_one_typed_action() {
    let mut app = fixture();
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::Form(WorkbenchFormKind::AddFolder)),
    );
    assert_eq!(
        workbench_action(&app, Input::Esc),
        Some(Action::Workbench(WorkbenchAction::Cancel))
    );
    assert!(text_paste_actions(&app, "/tmp\nquit".into(), TextAreaPasteSource::Clipboard).is_err());
    update(&mut app, Action::Workbench(WorkbenchAction::Cancel));
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::Focus(WorkbenchFocus::Editor)),
    );
    let actions = text_paste_actions(&app, "q\nb\n".into(), TextAreaPasteSource::BracketedPaste)
        .unwrap()
        .unwrap();
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0],
        Action::Workbench(WorkbenchAction::Edit(PopupEditorCommand::PasteText { .. }))
    ));
}
#[test]
fn workbench_terminal_keeps_control_keys_for_shell_and_exposes_explicit_exit_chord() {
    let mut app = fixture();
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::Focus(WorkbenchFocus::Terminal)),
    );
    for key in [
        Input::Esc,
        Input::CtrlC,
        Input::CtrlS,
        Input::CtrlF,
        Input::Tab,
        Input::Char('/'),
    ] {
        assert_eq!(workbench_action(&app, key), None);
    }
    assert_eq!(
        workbench_action(&app, Input::Alt('e')),
        Some(Action::Workbench(WorkbenchAction::Focus(
            WorkbenchFocus::Editor
        )))
    );
}

#[test]
fn workbench_terminal_resize_matches_bottom_panel_and_expanded_panel() {
    let mut app = fixture();
    app.daemon.pty_sessions.push(ClientDaemonPtySummary {
        id: 7,
        name: "workbench shell".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        viewers: 1,
    });
    app.pty_selection = 0;
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::Focus(WorkbenchFocus::Terminal)),
    );
    assert_eq!(
        terminal_workspace_dimensions(&app, 160, 50),
        Some(PtyDimensions {
            columns: 98,
            rows: 5
        })
    );
    app.workbench.current_mut().unwrap().panel_zoom = true;
    assert_eq!(
        terminal_workspace_dimensions(&app, 160, 50),
        Some(PtyDimensions {
            columns: 98,
            rows: 35
        })
    );
}
