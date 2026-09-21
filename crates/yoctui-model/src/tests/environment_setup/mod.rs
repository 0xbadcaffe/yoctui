use super::*;
fn action(app: &mut App, a: EnvironmentSetupAction) -> Option<Effect> {
    update(app, Action::EnvironmentSetup(a))
}
fn setup(app: &App) -> &EnvironmentSetup {
    let Some(Dialog::EnvironmentSetup(s)) = app.active_dialog() else {
        panic!("setup missing")
    };
    s
}
mod environment_setup_browser_rejects_stale_results_and_keeps_selection_bounded;
mod environment_setup_manual_save_cancel_and_validation;

mod environment_setup_manual_unicode_bounds_and_browser_errors_preserve_draft;
