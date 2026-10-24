use yoctui_app::*;
use yoctui_model::*;

fn fixture() -> App {
    let mut app = App::new(32, 8192);
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::OpenRecipe {
            recipe: "demo".into(),
            root: "/source".into(),
        }),
    );
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::Focus(WorkbenchFocus::Editor)),
    );
    app
}
#[test]
fn workbench_shortcuts_are_contextual_and_terminal_control_stays_with_the_shell() {
    let mut app = fixture();
    assert_eq!(inspector_toggle_action(&app, Input::Alt('i')), None);
    for (key, action) in [
        (Input::Alt('c'), WorkbenchAction::Compile),
        (Input::Alt('w'), WorkbenchAction::ToggleWhitespace),
        (Input::Alt('g'), WorkbenchAction::Debug),
        (Input::F5, WorkbenchAction::Debug),
        (Input::F9, WorkbenchAction::ToggleBreakpoint),
        (Input::Alt('9'), WorkbenchAction::SelectTab(8)),
        (
            Input::CtrlG,
            WorkbenchAction::SearchMatch { backwards: false },
        ),
        (
            Input::Alt('n'),
            WorkbenchAction::SearchMatch { backwards: false },
        ),
    ] {
        assert_eq!(workbench_action(&app, key), Some(Action::Workbench(action)));
    }
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::Focus(WorkbenchFocus::Terminal)),
    );
    for key in [Input::Alt('b'), Input::CtrlF, Input::F5, Input::Char('n')] {
        assert_eq!(workbench_action(&app, key), None);
    }
}
#[test]
fn workbench_debug_and_tab_dialogs_trap_paste_without_editing_hidden_source() {
    let mut app = fixture();
    update(&mut app, Action::Workbench(WorkbenchAction::Debug));
    assert!(!text_paste_active(&app), "Mode field is not an editor");
    update(&mut app, Action::Workbench(WorkbenchAction::DebugField(1)));
    let actions = text_paste_actions(
        &app,
        "/sdk/bin/cross-gdb".into(),
        TextAreaPasteSource::Clipboard,
    )
    .unwrap()
    .unwrap();
    assert!(
        actions
            .iter()
            .all(|a| matches!(a, Action::Workbench(WorkbenchAction::DebugText(_))))
    );
    assert!(text_paste_actions(&app, "path\nrun".into(), TextAreaPasteSource::Clipboard).is_err());
    assert_eq!(
        workbench_action(&app, Input::Tab),
        Some(Action::Workbench(WorkbenchAction::DebugField(1)))
    );
}
