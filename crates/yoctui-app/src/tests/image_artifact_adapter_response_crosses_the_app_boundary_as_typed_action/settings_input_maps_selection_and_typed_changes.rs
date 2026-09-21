use super::*;

#[test]
fn settings_input_maps_selection_and_typed_changes() {
    assert_eq!(
        settings_action(Input::Up),
        Some(Action::SelectSetting { delta: -1 })
    );
    assert_eq!(
        settings_action(Input::Down),
        Some(Action::SelectSetting { delta: 1 })
    );
    assert_eq!(
        settings_action(Input::Left),
        Some(Action::ChangeSelectedSetting { backwards: true })
    );
    assert_eq!(
        settings_action(Input::Enter),
        Some(Action::ChangeSelectedSetting { backwards: false })
    );
    assert_eq!(
        settings_action(Input::Char('r')),
        Some(Action::RetrySettingsPersistence)
    );
    assert_eq!(settings_action(Input::Esc), None);
}
