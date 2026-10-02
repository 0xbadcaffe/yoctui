use super::*;

#[test]
fn startup_screen_uses_saved_configured_destination_and_defaults_without_execution() {
    for (saved, expected) in [
        (None, Screen::Dashboard),
        (Some(Screen::Kernel), Screen::Kernel),
        (Some(Screen::Firmware), Screen::Firmware),
        (Some(Screen::Images), Screen::Images),
    ] {
        let mut app = App::new(16, 4096);
        restore_startup_screen(&mut app, true, saved);
        assert_eq!(app.screen, expected);
        assert_eq!(app.navigator_screen(), expected);
        assert_eq!(app.focus, yoctui_model::FocusTarget::Navigator);
        assert!(matches!(
            app.kernel.inventory,
            yoctui_model::PlatformInventoryState::NotLoaded
        ));
        assert!(matches!(
            app.firmware.inventory,
            yoctui_model::PlatformInventoryState::NotLoaded
        ));
        assert!(matches!(
            app.image_artifacts,
            yoctui_model::ImageArtifactInventoryState::NotLoaded
        ));
        assert!(app.active_dialog().is_none());
    }
}

#[test]
fn startup_screen_unconfigured_override_precedes_first_run_modal_onboarding() {
    for saved in [None, Some(Screen::Kernel), Some(Screen::BuildHistory)] {
        let mut app = App::new_unconfigured(16, 4096);
        restore_startup_screen(&mut app, false, saved);
        assert_eq!(app.screen, Screen::BuildEnvironment);
        assert_eq!(app.navigator_screen(), Screen::BuildEnvironment);
        assert_eq!(app.focus, yoctui_model::FocusTarget::Navigator);
        install_session_onboarding(&Session::default(), &mut app).unwrap();
        assert!(app.onboarding.open);
        assert_eq!(app.focus, yoctui_model::FocusTarget::Dialog);
        assert_eq!(app.screen, Screen::BuildEnvironment);
        assert_eq!(app.navigator_screen(), Screen::BuildEnvironment);
    }
}
