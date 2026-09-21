use super::*;

#[test]
fn raw_argv_tokenizes_quotes_escapes_empty_elements_and_unicode_as_native_arguments() {
    assert_eq!(
        parse("--flag plain 'single value' \"double value\" escaped\\ space '' \"\" café").unwrap(),
        [
            "--flag",
            "plain",
            "single value",
            "double value",
            "escaped space",
            "",
            "",
            "café",
        ]
    );
    assert_eq!(parse("").unwrap(), Vec::<String>::new());
    assert_eq!(
        parse(r#"a\b \"quoted\" slash\\value"#).unwrap(),
        ["ab", "\"quoted\"", "slash\\value",]
    );
}
