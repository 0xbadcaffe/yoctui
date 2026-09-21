use super::*;

#[test]
fn raw_form_editor_is_typed_bounded_and_invalidates_stale_values() {
    let catalog = catalog(1);
    let authority = authority(5, true);
    let mut state = RawModeState::new(&catalog);
    select_build_category(&mut state, &catalog);
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::OpenSelected,
    );
    let target = parameter("target");
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::EditParameterInput {
            parameter: target.clone(),
            command: PopupEditorCommand::ToggleInsert,
        },
    );
    for character in "busybox".chars() {
        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&authority),
            RawModeAction::EditParameterInput {
                parameter: target.clone(),
                command: PopupEditorCommand::Insert(character),
            },
        );
    }
    let field = &state.form.as_ref().unwrap().fields[&target];
    assert_eq!(field.editor.text, "busybox");
    assert_eq!(
        field.value,
        Some(RawParameterValue::Target("busybox".into()))
    );

    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::EditParameterInput {
            parameter: target.clone(),
            command: PopupEditorCommand::Insert('é'),
        },
    );
    let field = &state.form.as_ref().unwrap().fields[&target];
    assert_eq!(field.editor.text, "busyboxé");
    assert!(field.value.is_none());
    assert!(field.validation_error.is_some());

    for _ in 0..MAX_RAW_TARGET_BYTES {
        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&authority),
            RawModeAction::EditParameterInput {
                parameter: target.clone(),
                command: PopupEditorCommand::Insert('a'),
            },
        );
    }
    assert_eq!(
        state.form.as_ref().unwrap().fields[&target]
            .editor
            .text
            .len(),
        MAX_RAW_TARGET_BYTES
    );
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::SelectFormField { delta: 1 },
    );
    assert!(!state.form.as_ref().unwrap().fields[&target].editor.editing);
    assert_eq!(state.form.as_ref().unwrap().field_selection, 1);
}
