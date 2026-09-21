use super::*;

#[test]
fn raw_parameter_rejects_shell_syntax_traversal_and_invalid_numbers() {
    for kind in [
        RawParameterKind::Recipe,
        RawParameterKind::Image,
        RawParameterKind::Target,
        RawParameterKind::Task,
        RawParameterKind::UserInterface,
        RawParameterKind::Text,
        RawParameterKind::Multiconfig,
    ] {
        let parameter = parameter(kind, RawParameterPresence::Required);
        assert!(parameter.parse_value("--option").is_err());
        assert!(parameter.parse_value("value;other").is_err());
        assert!(parameter.parse_value("value\nother").is_err());
    }

    let file = parameter(RawParameterKind::File, RawParameterPresence::Required);
    for invalid in [
        "../recipe.bb",
        "layers/../recipe.bb",
        "./recipe.bb",
        "events$(date).json",
        "-events.json",
        "events.json/",
    ] {
        assert!(file.parse_value(invalid).is_err(), "{invalid}");
    }

    let integer = parameter(RawParameterKind::Integer, RawParameterPresence::Required);
    assert!(integer.parse_value("-1").is_err());
    assert!(integer.parse_value("1.5").is_err());
    assert_eq!(
        integer.parse_value("4294967296"),
        Err(integer.invalid(RawParameterInvalidReason::IntegerOutOfRange))
    );
}
