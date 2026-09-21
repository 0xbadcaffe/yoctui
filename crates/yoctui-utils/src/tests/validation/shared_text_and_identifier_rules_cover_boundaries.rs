use super::*;

#[test]
fn shared_text_and_identifier_rules_cover_boundaries() {
    assert!(is_bounded_plain_text("image", 5));
    assert!(!is_bounded_plain_text("", 5));
    assert!(!is_bounded_plain_text("bad\n", 5));
    assert!(is_bounded_identifier("core-image_minimal+dev", 256));
    for invalid in ["", ".", "..", "has space", "λ"] {
        assert!(!is_bounded_identifier(invalid, 256));
    }
}
