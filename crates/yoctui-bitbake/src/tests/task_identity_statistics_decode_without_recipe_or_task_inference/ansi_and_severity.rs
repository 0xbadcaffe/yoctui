use super::*;

#[test]
fn ansi_and_severity() {
    assert_eq!(strip_ansi("\x1b[31merror: bad\x1b[0m"), "error: bad");
    assert_eq!(
        classify_output("WARNING: x".into()).severity,
        Severity::Warning
    )
}
