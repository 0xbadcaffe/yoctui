use super::*;

#[test]
fn inspector_mode_tracks_typed_screen_selection_and_navigator_focus() {
    let mut app = App::new(10, 1_000);
    app.focus = FocusTarget::Workspace;
    for (screen, mode) in [
        (Screen::Dashboard, InspectorMode::DaemonSession),
        (Screen::Tasks, InspectorMode::Task),
        (Screen::BuildHistory, InspectorMode::Job),
        (Screen::Dependencies, InspectorMode::Dependency),
        (Screen::Recipes, InspectorMode::Recipe),
        (Screen::Packages, InspectorMode::Package),
        (Screen::Images, InspectorMode::Artifact),
        (Screen::Testing, InspectorMode::Test),
        (Screen::Maintenance, InspectorMode::Utility),
        (Screen::Errors, InspectorMode::Error),
        (
            Screen::Compatibility,
            InspectorMode::CompatibilityCapability,
        ),
    ] {
        app.screen = screen;
        assert_eq!(app.inspector_mode(), mode);
    }

    app.screen = Screen::Layers;
    assert_eq!(app.inspector_mode(), InspectorMode::Layer);
    let mut browser = LayerBrowser::new("meta-test".into(), PathBuf::from("/layers/meta-test"));
    browser.entries.push(LayerBrowserEntry {
        path: PathBuf::from("recipes-test/test.bb"),
        ..LayerBrowserEntry::default()
    });
    app.layer_browser = Some(browser);
    assert_eq!(app.inspector_mode(), InspectorMode::File);

    app.focus = FocusTarget::Navigator;
    assert_eq!(app.inspector_mode(), InspectorMode::Navigator);
    assert_eq!(
        InspectorMode::CompatibilityCapability.label(),
        "Compatibility capability"
    );
}
