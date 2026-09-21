use super::*;

#[test]
fn build_environment_input_verifies_and_returns_to_dashboard() {
    assert_eq!(
        build_environment_action(Input::Char('V')),
        Some(Action::BeginBuildEnvironmentVerification)
    );
    assert_eq!(
        build_environment_action(Input::Esc),
        Some(Action::Open(Screen::Dashboard))
    );
}
