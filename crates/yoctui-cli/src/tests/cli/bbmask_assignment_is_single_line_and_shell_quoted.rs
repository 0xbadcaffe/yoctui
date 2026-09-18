use super::*;

#[test]
fn bbmask_assignment_is_single_line_and_shell_quoted() {
    assert_eq!(
        bbmask_assignment("meta-broken/.* \"quoted\"").unwrap(),
        "BBMASK = \"meta-broken/.* \\\"quoted\\\"\""
    );
    assert!(bbmask_assignment("bad\nvalue").is_err());
}
