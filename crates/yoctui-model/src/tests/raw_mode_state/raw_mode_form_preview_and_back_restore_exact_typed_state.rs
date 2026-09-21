use super::*;

#[test]
fn raw_mode_form_preview_and_back_restore_exact_typed_state() {
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
    assert_eq!(state.view, RawModeView::Form);
    assert_eq!(state.focus, RawModeFocus::Form);

    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::RequestPreview,
    );
    assert_eq!(state.view, RawModeView::Form);
    assert!(
        state.form.as_ref().unwrap().fields[&parameter("target")]
            .validation_error
            .is_some()
    );

    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::ChooseParameter {
            parameter: parameter("target"),
            value: RawParameterValue::Target("virtual/kernel".into()),
        },
    );
    state
        .form
        .as_mut()
        .unwrap()
        .additional_arguments
        .replace_input("--verbose")
        .unwrap();
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::RequestPreview,
    );
    assert_eq!(state.view, RawModeView::Preview);
    assert_eq!(
        state.preview.as_ref().unwrap().arguments,
        ["virtual/kernel", "--verbose"]
    );

    reduce_raw_mode(&mut state, &catalog, Some(&authority), RawModeAction::Back);
    assert_eq!(state.view, RawModeView::Form);
    assert_eq!(
        state.form.as_ref().unwrap().fields[&parameter("target")].value,
        Some(RawParameterValue::Target("virtual/kernel".into()))
    );
    reduce_raw_mode(&mut state, &catalog, Some(&authority), RawModeAction::Back);
    assert_eq!(state.view, RawModeView::Browser);
    assert_eq!(state.focus, RawModeFocus::Commands);
}
