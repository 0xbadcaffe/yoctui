use super::*;

#[test]
fn raw_parameter_enforces_identifier_and_unicode_byte_boundaries() {
    for (kind, maximum) in [
        (RawParameterKind::Recipe, MAX_RAW_RECIPE_BYTES),
        (RawParameterKind::Image, MAX_RAW_IMAGE_BYTES),
        (RawParameterKind::Target, MAX_RAW_TARGET_BYTES),
        (RawParameterKind::Task, MAX_RAW_TASK_BYTES),
        (RawParameterKind::UserInterface, MAX_RAW_UI_BYTES),
        (RawParameterKind::Multiconfig, MAX_RAW_MULTICONFIG_BYTES),
    ] {
        assert!(
            parameter(kind, RawParameterPresence::Required)
                .parse_value(&"a".repeat(maximum))
                .is_ok()
        );
        assert!(
            parameter(kind, RawParameterPresence::Required)
                .parse_value(&"a".repeat(maximum + 1))
                .is_err()
        );
    }

    let text = parameter(RawParameterKind::Text, RawParameterPresence::Required);
    let exact_unicode = "é".repeat(MAX_RAW_PARAMETER_TEXT_BYTES / 2);
    assert_eq!(exact_unicode.len(), MAX_RAW_PARAMETER_TEXT_BYTES);
    assert!(text.parse_value(&exact_unicode).is_ok());
    assert!(text.parse_value(&(exact_unicode + "é")).is_err());
    assert!(
        parameter(RawParameterKind::Recipe, RawParameterPresence::Required)
            .parse_value("récipe")
            .is_err()
    );

    let file = parameter(RawParameterKind::File, RawParameterPresence::Required);
    assert!(file.parse_value("レイヤ/recipe.bb").is_ok());
    assert!(
        file.parse_value(&format!("/{}", "a".repeat(MAX_RAW_FILE_BYTES - 1)))
            .is_ok()
    );
    assert!(
        file.parse_value(&format!("/{}", "a".repeat(MAX_RAW_FILE_BYTES)))
            .is_err()
    );
}
