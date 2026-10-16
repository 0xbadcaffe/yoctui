use super::*;
use crate::key_action;
use yoctui_model::{FileDiffAction as Diff, FileDiffComparison};

#[test]
fn file_diff_context_owns_compare_tab_and_two_stage_escape_only_in_workspace() {
    let mut app = App::new(16, 4096);
    yoctui_model::update(&mut app, Action::Open(Screen::DiffTool));
    for (key, action) in [
        (Input::F5, Diff::Compare),
        (Input::Tab, Diff::SwitchPane),
        (Input::BackTab, Diff::SwitchPane),
        (Input::Char(' '), Diff::Mark),
        (Input::Esc, Diff::Back),
        (Input::Right, Diff::Expand),
    ] {
        assert_eq!(file_diff_action(&app, key), Some(Action::FileDiff(action)));
    }
    app.file_diff.comparison = Some(FileDiffComparison {
        paths: Default::default(),
        rows: vec![],
        identical: true,
        final_newline: [false; 2],
        changed_rows: 0,
    });
    assert_eq!(
        file_diff_action(&app, Input::Right),
        Some(Action::FileDiff(Diff::Horizontal { delta: 8 }))
    );
    app.focus = FocusTarget::Navigator;
    assert_eq!(file_diff_action(&app, Input::F5), None);
    assert_eq!(key_action(Input::F5), Some(Action::Open(Screen::Logs)));
    app.screen = Screen::Layers;
    app.focus = FocusTarget::Workspace;
    assert_eq!(file_diff_action(&app, Input::Tab), None);
}

#[test]
fn file_diff_menu_and_help_overlays_keep_ownership() {
    let mut app = App::new(16, 4096);
    yoctui_model::update(&mut app, Action::Open(Screen::DiffTool));
    yoctui_model::update(&mut app, Action::OpenApplicationMenu);
    assert_eq!(file_diff_action(&app, Input::F5), None);
    yoctui_model::update(&mut app, Action::CloseMenu);
    yoctui_model::update(&mut app, Action::Open(Screen::Help));
    assert_eq!(
        help_workspace_action(&app, Input::Esc),
        Some(Action::Open(Screen::Dashboard))
    );
    assert_eq!(
        help_workspace_action(&app, Input::Char('g')),
        Some(Action::OpenOnboarding)
    );
    assert_eq!(
        help_workspace_action(&app, Input::End),
        Some(Action::ScrollHelp { delta: isize::MAX })
    );
    yoctui_model::update(&mut app, Action::OpenAbout);
    assert_eq!(help_workspace_action(&app, Input::End), None);
    assert_eq!(
        help_workspace_action(&app, Input::Char('g')),
        Some(Action::OpenOnboarding)
    );
    yoctui_model::update(&mut app, Action::OpenOnboarding);
    assert_eq!(help_workspace_action(&app, Input::End), None);
}
