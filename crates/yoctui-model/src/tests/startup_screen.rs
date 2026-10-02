use super::*;

#[test]
fn startup_screen_restoration_aligns_every_catalog_destination_without_loading() {
    for screen in NAVIGATOR_SCREENS {
        let mut app = App::new(16, 4096);
        app.focus = FocusTarget::Workspace;
        app.navigator_selection = usize::MAX;
        app.restore_startup_screen(screen);
        assert_eq!(app.screen, screen);
        assert_eq!(app.navigator_screen(), screen);
        assert_eq!(app.focus, FocusTarget::Navigator);
        assert_eq!(
            app.navigator_selection,
            NAVIGATOR_SCREENS.iter().position(|s| *s == screen).unwrap()
        );
        assert!(matches!(
            app.kernel.inventory,
            PlatformInventoryState::NotLoaded
        ));
        assert!(matches!(
            app.firmware.inventory,
            PlatformInventoryState::NotLoaded
        ));
        assert!(matches!(
            app.image_artifacts,
            ImageArtifactInventoryState::NotLoaded
        ));
        assert!(matches!(
            app.package_inventory,
            PackageInventoryState::NotLoaded
        ));
        assert!(!app.saved_builds.reload_requested);
        assert!(app.active_dialog().is_none());
        assert!(app.build.target.is_none());
    }
}

#[test]
fn startup_screen_restoration_preserves_non_catalog_screens_with_default_context() {
    for screen in [
        Screen::Help,
        Screen::BuildHistory,
        Screen::Signatures,
        Screen::LayerRelationships,
        Screen::Bbmask,
    ] {
        let mut app = App::new(16, 4096);
        app.navigator_selection = 7;
        app.restore_startup_screen(screen);
        assert_eq!(app.screen, screen);
        assert_eq!(app.navigator_screen(), Screen::Dashboard);
        assert_eq!(app.focus, FocusTarget::Navigator);
        assert!(!app.saved_builds.reload_requested);
    }
}
