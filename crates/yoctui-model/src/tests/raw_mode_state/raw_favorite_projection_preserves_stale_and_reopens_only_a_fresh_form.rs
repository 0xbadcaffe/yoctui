use super::*;

#[test]
fn raw_favorite_projection_preserves_stale_and_reopens_only_a_fresh_form() {
    let original = catalog(1);
    let favorite = favorite_fixture(&original, "build.target", 0);
    let available = authority(7, true);
    let projection = favorite.project(&original, Some(&available));
    assert!(!projection.stale);
    assert_eq!(
        projection.availability.state,
        RawAvailabilityState::Available
    );

    let mut changed = catalog(2);
    let RawExecutionPolicy::Executable { template } = &mut changed
        .commands
        .iter_mut()
        .find(|item| item.id == command("build.target"))
        .unwrap()
        .execution
    else {
        unreachable!();
    };
    template.arguments.push(RawArgument::Literal {
        value: "--changed".into(),
    });
    assert!(favorite.project(&changed, Some(&available)).stale);
    changed
        .commands
        .retain(|item| item.id != command("build.target"));
    assert!(favorite.project(&changed, Some(&available)).stale);

    let mut state = RawModeState::new(&original);
    state.favorites = vec![favorite.clone()];
    state.view = RawModeView::Favorites;
    state.focus = RawModeFocus::Favorites;
    reduce_raw_mode(
        &mut state,
        &original,
        Some(&available),
        RawModeAction::ActivateFavorite,
    );
    assert_eq!(state.view, RawModeView::Form);
    let form = state.form.as_mut().unwrap();
    assert_eq!(form.capability_generation, 7);
    assert_eq!(
        form.fields[&parameter("target")].value,
        Some(RawParameterValue::Target("core-image-minimal".into()))
    );
    assert_eq!(
        form.additional_arguments.validate().unwrap(),
        &favorite.additional_arguments
    );
    assert!(state.preview.is_none());
    assert!(state.execution_states.is_empty());
}
