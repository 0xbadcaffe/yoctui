use super::*;
use yoctui_model::{
    Action, App, BuildEnvironmentProfile, Dialog, FocusTarget, SavedBuildAction,
    SavedEnvironmentAction, SavedEnvironmentMode, SavedEnvironmentPlan, Screen,
};

#[test]
fn saved_environment_input_loads_from_list_or_details_and_traps_review() {
    let mut app = App::new(32, 4096);
    app.screen = Screen::BuildHistory;
    app.focus = FocusTarget::Workspace;
    app.saved_builds.browsing = true;
    assert_eq!(
        saved_build_workspace_action(&app, crate::Input::Char('o')),
        Some(Action::SavedBuild(SavedBuildAction::Environment(
            SavedEnvironmentAction::Begin
        )))
    );
    app.saved_builds.view = Some(yoctui_model::SavedBuildView::Summary);
    assert!(saved_build_workspace_action(&app, crate::Input::Char('o')).is_some());
    app.focus = FocusTarget::Navigator;
    assert!(saved_build_workspace_action(&app, crate::Input::Char('o')).is_none());
    app.dialogs
        .push_back(Dialog::SavedEnvironmentReview(SavedEnvironmentPlan {
            profile: BuildEnvironmentProfile {
                source_dir: "/source".into(),
                build_dir: "/build".into(),
                init_script: "/source/oe-init-build-env".into(),
            },
            target: "image".into(),
            machine: None,
            mode: SavedEnvironmentMode::Start,
        }));
    for input in [crate::Input::Enter, crate::Input::Char('y')] {
        assert_eq!(
            saved_build_workspace_action(&app, input),
            Some(Action::SavedBuild(SavedBuildAction::Environment(
                SavedEnvironmentAction::Confirm
            )))
        );
    }
    assert_eq!(
        saved_build_workspace_action(&app, crate::Input::Esc),
        Some(Action::SavedBuild(SavedBuildAction::Environment(
            SavedEnvironmentAction::Cancel
        )))
    );
    assert_eq!(
        saved_build_workspace_action(&app, crate::Input::Tab),
        Some(Action::Focus(FocusTarget::Dialog))
    );
}

#[test]
fn saved_environment_loading_holds_work_and_keeps_exit_confirmation() {
    let mut app = App::new(32, 4096);
    app.saved_builds.environment.loading = true;
    assert!(matches!(
        saved_build_workspace_action(&app, crate::Input::Char('b')),
        Some(Action::Notify(_))
    ));
    assert_eq!(
        saved_build_workspace_action(&app, crate::Input::CtrlC),
        Some(Action::Quit)
    );
    app.dialogs.push_back(Dialog::QuitConfirmation);
    assert_eq!(
        saved_build_workspace_action(&app, crate::Input::Enter),
        Some(Action::ConfirmQuit)
    );
    assert_eq!(
        saved_build_workspace_action(&app, crate::Input::Esc),
        Some(Action::CancelQuit)
    );
}

#[test]
fn saved_environment_owned_failure_can_retry_without_dismissing_other_notices() {
    let mut app = App::new(32, 4096);
    app.screen = Screen::BuildHistory;
    app.focus = FocusTarget::Workspace;
    app.saved_builds.browsing = true;
    app.saved_builds.environment.error = Some("missing source".into());
    app.notification = Some("Load environment failed: missing source".into());
    assert!(matches!(
        saved_build_workspace_action(&app, crate::Input::Char('o')),
        Some(Action::SavedBuild(SavedBuildAction::Environment(
            SavedEnvironmentAction::Begin
        )))
    ));
    app.notification = Some("Select a file first".into());
    assert!(saved_build_workspace_action(&app, crate::Input::Char('o')).is_none());
}
