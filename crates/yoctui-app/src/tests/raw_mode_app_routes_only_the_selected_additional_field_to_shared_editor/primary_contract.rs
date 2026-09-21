use super::*;

#[test]
fn raw_mode_app_routes_only_the_selected_additional_field_to_shared_editor() {
    let catalog = yoctui_model::RawCatalog::builtin();
    let mut state = yoctui_model::RawModeState::new(&catalog);
    state.view = yoctui_model::RawModeView::Form;
    state.focus = yoctui_model::RawModeFocus::Form;
    state.form = Some(yoctui_model::RawCommandForm {
        command: catalog.commands[0].id.clone(),
        fields: std::collections::BTreeMap::new(),
        field_order: Vec::new(),
        field_selection: 0,
        additional_arguments: yoctui_model::RawArgvEditor::new("").unwrap(),
        capability_generation: 1,
        build_directory: "/work/build".into(),
    });

    assert_eq!(
        raw_mode_input(&state, Input::Char('i')),
        Some(yoctui_model::RawModeAction::EditAdditionalArguments(
            yoctui_model::PopupEditorCommand::ToggleInsert
        ))
    );
    state
        .form
        .as_mut()
        .unwrap()
        .additional_arguments
        .editor
        .editing = true;
    assert_eq!(
        raw_mode_input(&state, Input::Char('v')),
        Some(yoctui_model::RawModeAction::EditAdditionalArguments(
            yoctui_model::PopupEditorCommand::Insert('v')
        ))
    );
    assert_eq!(
        raw_mode_input(&state, Input::Enter),
        Some(yoctui_model::RawModeAction::RequestPreview)
    );
    assert_eq!(raw_mode_input(&state, Input::CtrlS), None);
}
