use super::*;

#[test]
fn raw_security_argv_rejects_every_documented_operator_even_when_quoted_or_assembled() {
    for (input, operator) in [
        ("'left|right'", "|"),
        ("left>right", ">"),
        ("left>>right", ">>"),
        ("left<right", "<"),
        ("left&&right", "&&"),
        ("left||right", "||"),
        ("left;right", ";"),
        ("'$('date')'", "$("),
        ("`date`", "`"),
        ("'$''('", "$("),
    ] {
        assert_eq!(
            parse(input),
            Err(RawArgvError::ForbiddenOperator {
                argument: 0,
                operator: operator.into(),
            }),
            "{input}"
        );
    }
    assert!(matches!(
        parse("left\\|right"),
        Err(RawArgvError::InvalidEscape { character: '|', .. })
    ));
    assert_eq!(
        parse("literal&value $HOME (literal)").unwrap(),
        ["literal&value", "$HOME", "(literal)"]
    );
}
