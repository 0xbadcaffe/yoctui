use super::*;

#[test]
fn ux_checkbox_states_markers_focus_and_disabled_reason_are_explicit() {
    let mut row = CheckboxState::new("pkg:a", "Package A");
    assert_eq!((row.marker(true), row.marker(false)), ("☐", "[ ]"));
    assert!(row.toggle());
    assert_eq!((row.marker(true), row.semantic_state()), ("☑", "checked"));
    row.value = CheckboxValue::Indeterminate;
    assert_eq!(
        (row.marker(false), row.semantic_state()),
        ("[-]", "indeterminate")
    );
    row.set_disabled("not available for this image");
    assert!(!row.toggle());
    assert_eq!(row.semantic_state(), "disabled");
}
