use super::*;

#[test]
fn yocto_utility_dialog_routes_navigation_choices_text_review_and_cancel() {
    let mut dialog =
        yoctui_model::YoctoUtilityDialog::new(yoctui_model::YoctoUtilityCommand::LayersCreateLayer);
    assert_eq!(
        yocto_utility_dialog_action(&dialog, Input::Tab),
        Some(Action::SelectYoctoUtilityField { delta: 1 })
    );
    assert_eq!(
        yocto_utility_dialog_action(&dialog, Input::BackTab),
        Some(Action::SelectYoctoUtilityField { delta: -1 })
    );
    assert_eq!(
        yocto_utility_dialog_action(&dialog, Input::Right),
        Some(Action::CycleYoctoUtilityChoice { delta: 1 })
    );
    assert_eq!(
        yocto_utility_dialog_action(&dialog, Input::Char('*')),
        Some(Action::AppendYoctoUtilityField('*'))
    );
    assert_eq!(
        yocto_utility_dialog_action(&dialog, Input::Char(' ')),
        Some(Action::AppendYoctoUtilityField(' '))
    );
    dialog.select_field(1);
    assert_eq!(
        yocto_utility_dialog_action(&dialog, Input::Char(' ')),
        Some(Action::CycleYoctoUtilityChoice { delta: 1 })
    );
    assert_eq!(
        yocto_utility_dialog_action(&dialog, Input::Enter),
        Some(Action::ReviewYoctoUtility)
    );
    assert_eq!(
        yocto_utility_dialog_action(&dialog, Input::Esc),
        Some(Action::CancelYoctoUtility)
    );
}
