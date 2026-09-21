use super::*;

#[test]
fn overlays_and_reduced_motion_freeze_animation_but_not_elapsed_time() {
    let mut app = App::new(16, 16 * 1024);
    app.build.status = BuildStatus::Running;
    assert!(has_live_elapsed_time(&app));

    app.reduced_motion = true;
    assert!(!has_visible_indeterminate_activity(&app));
    assert!(has_live_elapsed_time(&app));

    app.reduced_motion = false;
    app.dialogs.push_back(Dialog::BuildCompletion);
    assert!(!has_visible_indeterminate_activity(&app));

    app.dialogs.clear();
    app.command_palette_open = true;
    assert!(!has_visible_indeterminate_activity(&app));
}
