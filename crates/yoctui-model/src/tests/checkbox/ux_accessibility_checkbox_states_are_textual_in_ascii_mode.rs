use super::*;

#[test]
fn ux_accessibility_checkbox_states_are_textual_in_ascii_mode() {
    let mut row = CheckboxState::new("pkg:a", "Package A");
    for (value, marker, state) in [
        (CheckboxValue::Unchecked, "[ ]", "unchecked"),
        (CheckboxValue::Checked, "[x]", "checked"),
        (CheckboxValue::Indeterminate, "[-]", "indeterminate"),
    ] {
        row.value = value;
        assert_eq!(row.marker(false), marker);
        assert_eq!(row.semantic_state(), state);
    }
    row.set_disabled("required dependency");
    assert_eq!(row.semantic_state(), "disabled");
    assert_eq!(row.disabled_reason.as_deref(), Some("required dependency"));
}
