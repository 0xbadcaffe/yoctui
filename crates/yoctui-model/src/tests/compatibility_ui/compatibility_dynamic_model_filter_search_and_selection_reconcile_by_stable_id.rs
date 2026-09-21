use super::*;

#[test]
fn compatibility_dynamic_model_filter_search_and_selection_reconcile_by_stable_id() {
    let first = authority(1);
    let compatibility = state_with(first.clone());
    let mut state = CompatibilityUiState::default();
    state.reconcile(compatibility.authority());
    state.set_filter(CompatibilityUiFilter::Attention, compatibility.authority());
    assert_eq!(
        state
            .project(&compatibility, ClientReplicaStatus::Current)
            .rows
            .len(),
        2
    );
    state.select(1, compatibility.authority());
    let selected = state.selected().unwrap();
    state.begin_search();
    for character in selected.as_str().chars() {
        assert!(state.append_query(character, compatibility.authority()));
    }
    assert_eq!(
        state
            .project(&compatibility, ClientReplicaStatus::Current)
            .selected,
        Some(selected)
    );

    let replacement = state_with(authority(2));
    state.reconcile(replacement.authority());
    assert_eq!(state.selected(), Some(selected));
    while !state.query.is_empty() {
        state.backspace_query(replacement.authority());
    }
    state.set_filter(CompatibilityUiFilter::Unavailable, replacement.authority());
    assert_eq!(state.selected(), Some(CapabilityId::DevtoolUpgrade));
    state.reconcile(None);
    assert_eq!(state.selected(), None);
}
