use super::*;

#[test]
fn invalid_utf8_output_is_preserved_lossily() {
    assert_eq!(output_text(b"warning: \xff\n"), "warning: �");
}
