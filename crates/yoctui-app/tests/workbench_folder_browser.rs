use yoctui_app::*;
use yoctui_model::*;

#[test]
fn folder_browser_separates_text_entry_and_list_navigation() {
    let mut app = App::new(32, 8192);
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::OpenRecipe {
            recipe: "busybox".into(),
            root: "/source".into(),
        }),
    );
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::Form(WorkbenchFormKind::AddFolder)),
    );
    assert_eq!(
        workbench_action(&app, Input::Char('j')),
        Some(Action::Workbench(WorkbenchAction::FormText('j')))
    );
    assert_eq!(
        workbench_action(&app, Input::Enter),
        Some(Action::Workbench(WorkbenchAction::ConfirmForm))
    );
    update(&mut app, Action::Workbench(WorkbenchAction::BrowseFolders));
    for (key, expected) in [
        (Input::Down, WorkbenchAction::BrowseSelect(1)),
        (Input::Left, WorkbenchAction::BrowseParent),
        (Input::Enter, WorkbenchAction::BrowseOpen),
        (Input::Char(' '), WorkbenchAction::BrowseChoose),
        (Input::End, WorkbenchAction::BrowseSelect(isize::MAX)),
        (Input::Esc, WorkbenchAction::Cancel),
    ] {
        assert_eq!(
            workbench_action(&app, key),
            Some(Action::Workbench(expected))
        );
    }
    assert_eq!(workbench_action(&app, Input::Char('q')), None);
}
