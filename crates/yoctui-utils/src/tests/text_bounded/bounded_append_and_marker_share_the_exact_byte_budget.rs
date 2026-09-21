use super::*;

#[test]
fn bounded_append_and_marker_share_the_exact_byte_budget() {
    let mut output = "a".to_owned();
    assert!(!push_bounded(&mut output, "λ🙂", 4));
    assert_eq!(output, "aλ");
    append_truncation_marker(&mut output, "!", 3);
    assert_eq!(output, "a!");
    append_truncation_marker(&mut output, "too long", 3);
    assert_eq!(output, "a!");
    assert!(push_bounded(&mut output, "b", 3));
    assert_eq!(output, "a!b");
    assert_eq!(utf8_prefix("λ🙂", usize::MAX), "λ🙂");
}
