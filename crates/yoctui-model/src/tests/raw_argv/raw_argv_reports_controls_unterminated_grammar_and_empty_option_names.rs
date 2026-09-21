use super::*;

#[test]
fn raw_argv_reports_controls_unterminated_grammar_and_empty_option_names() {
    for input in ["line\nbreak", "tab\tvalue", "nul\0value"] {
        assert!(matches!(
            parse(input),
            Err(RawArgvError::ControlCharacter { .. })
        ));
    }
    assert!(matches!(
        parse("'open"),
        Err(RawArgvError::UnterminatedQuote { quote: '\'', .. })
    ));
    assert!(matches!(
        parse("open\\"),
        Err(RawArgvError::UnterminatedEscape { .. })
    ));
    for input in ["--=value", "-=value"] {
        assert_eq!(
            parse(input),
            Err(RawArgvError::EmptyOptionName { argument: 0 })
        );
    }
    assert_eq!(parse("-- -").unwrap(), ["--", "-"]);
}
