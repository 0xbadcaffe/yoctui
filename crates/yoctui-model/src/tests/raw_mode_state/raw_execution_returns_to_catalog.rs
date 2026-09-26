use super::*;

#[test]
fn confirmed_execution_closes_directly_to_the_command_catalog() {
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
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::ChooseParameter {
            parameter: parameter("target"),
            value: RawParameterValue::Target("core-image-minimal".into()),
        },
    );
    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::RequestPreview,
    );
    assert_eq!(state.view, RawModeView::Preview);

    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::OpenExecution(command("build.target")),
    );
    assert_eq!(state.view, RawModeView::Execution);
    assert!(state.form.is_none());
    assert!(state.preview.is_none());

    reduce_raw_mode(
        &mut state,
        &catalog,
        Some(&authority),
        RawModeAction::CloseExecution,
    );
    assert_eq!(state.view, RawModeView::Browser);
    assert_eq!(state.browser_column, RawBrowserColumn::Commands);
    assert_eq!(state.focus, RawModeFocus::Commands);
}
