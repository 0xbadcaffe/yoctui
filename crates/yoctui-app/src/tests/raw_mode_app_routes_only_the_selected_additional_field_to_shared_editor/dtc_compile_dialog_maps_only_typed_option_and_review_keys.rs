use super::*;

#[test]
fn dtc_compile_dialog_maps_only_typed_option_and_review_keys() {
    assert_eq!(
        dtc_compile_dialog_action(Input::Up),
        Some(Action::SelectDtcCompileOption { delta: -1 })
    );
    assert_eq!(
        dtc_compile_dialog_action(Input::Tab),
        Some(Action::SelectDtcCompileOption { delta: 1 })
    );
    assert_eq!(
        dtc_compile_dialog_action(Input::Char(' ')),
        Some(Action::AdjustDtcCompileOption { delta: 1 })
    );
    assert_eq!(
        dtc_compile_dialog_action(Input::Left),
        Some(Action::AdjustDtcCompileOption { delta: -1 })
    );
    assert_eq!(
        dtc_compile_dialog_action(Input::Enter),
        Some(Action::ConfirmDtcCompileOptions)
    );
    assert_eq!(
        dtc_compile_dialog_action(Input::Esc),
        Some(Action::CancelDtcCompileOptions)
    );
    assert_eq!(dtc_compile_dialog_action(Input::Char('x')), None);
}
