use super::*;
use yoctui_model::{BuildEnvironmentProfile, Dialog, SavedEnvironmentMode, SavedEnvironmentPlan};

#[test]
fn saved_environment_review_shows_exact_paths_and_safe_narrow_rendering() {
    let mut app = App::new(32, 4096);
    app.dialogs
        .push_back(Dialog::SavedEnvironmentReview(SavedEnvironmentPlan {
            profile: BuildEnvironmentProfile {
                source_dir: "/yocto/source".into(),
                build_dir: "/yocto/build".into(),
                init_script: "/yocto/source/oe-init-build-env".into(),
            },
            target: "historical-image".into(),
            machine: Some("romulus".into()),
            mode: SavedEnvironmentMode::Restart { instance: [1; 16] },
        }));
    app.focus = FocusTarget::Dialog;
    let text = rendered_text(&app, 160, 42);
    for expected in [
        "Load saved environment",
        "/yocto/source",
        "/yocto/build",
        "oe-init-build-env",
        "historical-image",
        "romulus",
        "Restart idle daemon",
        "No saved build",
        "[y/Enter] Load environment",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    for (width, height) in [(1, 1), (20, 5), (60, 15)] {
        let _ = rendered_text(&app, width, height);
    }
}

#[test]
fn saved_environment_history_names_preparation_and_loading() {
    let mut app = App::new(32, 4096);
    app.screen = Screen::BuildHistory;
    app.saved_builds.browsing = true;
    app.focus = FocusTarget::Workspace;
    assert!(rendered_text(&app, 160, 42).contains("o Load environment"));
    app.saved_builds.environment.preparing = true;
    assert!(rendered_text(&app, 160, 42).contains("Preparing saved environment"));
    app.saved_builds.environment.preparing = false;
    app.saved_builds.environment.loading = true;
    assert!(rendered_text(&app, 160, 42).contains("Loading saved environment"));
}
