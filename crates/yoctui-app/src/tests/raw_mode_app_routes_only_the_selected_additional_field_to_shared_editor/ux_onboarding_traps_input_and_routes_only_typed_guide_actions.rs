use super::*;

#[test]
fn ux_onboarding_traps_input_and_routes_only_typed_guide_actions() {
    let mut app = yoctui_model::App::new_unconfigured(8, 1_000);
    assert_eq!(onboarding_action(&app, Input::Enter), None);
    let _ = yoctui_model::update(&mut app, Action::OpenOnboarding);

    assert_eq!(
        onboarding_action(&app, Input::Down),
        Some(Action::SelectOnboarding { delta: 1 })
    );
    assert_eq!(
        onboarding_action(&app, Input::Enter),
        Some(Action::ActivateOnboardingStep)
    );
    assert_eq!(
        onboarding_action(&app, Input::Char('n')),
        Some(Action::AdvanceOnboarding)
    );
    assert_eq!(
        onboarding_action(&app, Input::Char('s')),
        Some(Action::SkipOnboardingStep)
    );
    assert_eq!(
        onboarding_action(&app, Input::Esc),
        Some(Action::DismissOnboarding)
    );

    assert!(matches!(
        keymap_action_for_app(&mut app, Input::Char('B')),
        KeymapInputResult::Unmatched
    ));
}
