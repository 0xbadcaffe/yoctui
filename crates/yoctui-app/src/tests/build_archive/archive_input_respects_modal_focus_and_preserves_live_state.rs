use super::*;

#[test]
fn archive_input_respects_modal_focus_and_preserves_live_state() {
    use yoctui_model::{Action, FocusTarget, Screen};
    let mut app = yoctui_model::App::new(32, 4096);
    app.screen = Screen::BuildHistory;
    app.focus = FocusTarget::Workspace;
    app.saved_builds.browsing = true;
    assert!(matches!(
        saved_build_workspace_action(&app, crate::Input::Enter),
        Some(Action::SavedBuild(_))
    ));
    app.command_palette_open = true;
    assert!(saved_build_workspace_action(&app, crate::Input::Enter).is_none());
}
