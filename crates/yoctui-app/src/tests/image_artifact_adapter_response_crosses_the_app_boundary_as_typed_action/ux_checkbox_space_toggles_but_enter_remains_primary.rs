use super::*;

#[test]
fn ux_checkbox_space_toggles_but_enter_remains_primary() {
    assert_eq!(
        checkbox_input_action(Input::Char(' ')),
        Some(CheckboxInputAction::Toggle)
    );
    assert_eq!(
        checkbox_input_action(Input::Enter),
        Some(CheckboxInputAction::Primary)
    );
    assert_eq!(
        checkbox_input_action(Input::Down),
        Some(CheckboxInputAction::Move(1))
    );
}
