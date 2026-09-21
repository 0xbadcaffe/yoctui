use super::*;

#[test]
fn raw_mode_capability_replacement_closes_stale_preview_or_unsafe_form() {
    let catalog = catalog(1);
    let first = authority(5, true);
    let mut state = RawModeState::new(&catalog);
    select_build_category(&mut state, &catalog);
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&first),
        RawModeAction::OpenSelected,
    );
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&first),
        RawModeAction::SetParameterInput {
            parameter: parameter("target"),
            input: "busybox".into(),
        },
    );
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&first),
        RawModeAction::RequestPreview,
    );
    assert_eq!(state.view, RawModeView::Preview);

    let replacement = authority(6, true);
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&replacement),
        RawModeAction::ReprojectAuthority,
    );
    assert_eq!(state.view, RawModeView::Form);
    assert!(state.preview.is_none());
    assert_eq!(state.form.as_ref().unwrap().capability_generation, 6);

    let unavailable = authority(7, false);
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&unavailable),
        RawModeAction::ReprojectAuthority,
    );
    assert_eq!(state.view, RawModeView::Browser);
    assert!(state.form.is_none());
    assert!(
        state
            .notification
            .as_deref()
            .is_some_and(|message| message.contains("Raw CLI probe is unavailable"))
    );
}
