use super::*;

#[test]
fn compatibility_ui_model_query_is_bounded_and_rejects_control_input() {
    let compatibility = state_with(authority(1));
    let mut state = CompatibilityUiState::default();
    assert!(!state.append_query('\n', compatibility.authority()));
    for _ in 0..MAX_COMPATIBILITY_UI_QUERY_BYTES {
        assert!(state.append_query('a', compatibility.authority()));
    }
    assert!(!state.append_query('b', compatibility.authority()));
    assert_eq!(state.query.len(), MAX_COMPATIBILITY_UI_QUERY_BYTES);
}
