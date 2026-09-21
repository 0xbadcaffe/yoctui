use super::*;

#[test]
fn config_source_picker_keys_are_modal_and_typed() {
    assert_eq!(
        config_source_picker_action(Input::Down),
        Some(Action::SelectConfigSource { delta: 1 })
    );
    assert_eq!(
        config_source_picker_action(Input::Enter),
        Some(Action::OpenSelectedConfigSourceChoice)
    );
    assert_eq!(
        config_source_picker_action(Input::Esc),
        Some(Action::CancelConfigSourcePicker)
    );
}
