use super::*;

#[test]
fn ansi_controls_do_not_swallow_text_after_punctuation_finals() {
    assert_eq!(strip_ansi("a\x1b[1~after\x1b[2@done"), "aafterdone");
    assert_eq!(strip_ansi("\x1b[31mλ warning\x1b[0m\n\t"), "λ warning\n\t");
    assert_eq!(
        strip_ansi("\x1b]8;;https://example.invalid\x1b\\link\x1b]8;;\x1b\\"),
        "link"
    );
    assert_eq!(strip_ansi("a\x1b]title\x07b\x1bPpayload\x1b\\c"), "abc");
    assert_eq!(strip_ansi("a\x1b[31"), "a");
    assert_eq!(strip_ansi("a\x1b]unfinished"), "a");
    assert_eq!(strip_ansi("a\x1b"), "a");
    assert_eq!(strip_ansi("\x1b(Btext"), "text");
}
