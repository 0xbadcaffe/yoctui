use super::*;

#[test]
fn signature_workspace_maps_recipe_entry_picker_and_workspace_keys() {
    assert_eq!(
        recipes_workspace_action(false, Input::Char('Z')),
        Some(Action::BeginSelectedRecipeSignatures)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('z')),
        Some(Action::BeginSelectedRecipeDiffsigs)
    );
    assert_eq!(
        signature_task_picker_action(Input::Down),
        Some(Action::SelectSignatureTask { delta: 1 })
    );
    assert_eq!(
        signature_task_picker_action(Input::Enter),
        Some(Action::ConfirmSignatureTask)
    );
    assert_eq!(
        signature_task_picker_action(Input::Esc),
        Some(Action::CancelSignatureTaskPicker)
    );
    assert_eq!(
        signature_workspace_action(Input::Up),
        Some(Action::SelectSignatureRecord { delta: -1 })
    );
    assert_eq!(
        signature_workspace_action(Input::Char('1')),
        Some(Action::SetSelectedSignatureComparisonSide(
            yoctui_model::SignatureComparisonSide::Left
        ))
    );
    assert_eq!(
        signature_workspace_action(Input::Char('2')),
        Some(Action::SetSelectedSignatureComparisonSide(
            yoctui_model::SignatureComparisonSide::Right
        ))
    );
    assert_eq!(
        signature_workspace_action(Input::Char('c')),
        Some(Action::BeginSignatureComparison)
    );
    assert_eq!(
        signature_workspace_action(Input::Char('r')),
        Some(Action::RefreshSignatureDump)
    );
    assert_eq!(
        signature_workspace_action(Input::Char('e')),
        Some(Action::OpenSignatureProvider)
    );
    assert_eq!(
        signature_workspace_action(Input::Esc),
        Some(Action::LeaveSignatureWorkspace)
    );
    assert_eq!(signature_workspace_action(Input::Char('x')), None);
}
