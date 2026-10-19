use yoctui_utils::{
    append_truncation_marker, is_csi_final_byte, push_bounded, strip_ansi, truncate_utf8,
    utf8_prefix,
};

#[test]
fn empty_text_survives_all_zero_budget_operations() {
    let mut text = String::new();
    truncate_utf8(&mut text, 0);
    assert!(push_bounded(&mut text, "", 0));
    append_truncation_marker(&mut text, "!", 0);
    assert_eq!(text, "");
    assert_eq!(utf8_prefix("", usize::MAX), "");
}

#[test]
fn prefix_is_the_longest_valid_prefix_for_every_byte_budget() {
    for input in ["", "ascii", "aλ茶🙂z", "e\u{301}", "👨‍👩‍👧‍👦"] {
        for budget in 0..=input.len() + 2 {
            let prefix = utf8_prefix(input, budget);
            assert!(input.starts_with(prefix));
            assert!(prefix.len() <= budget);
            assert!(
                prefix.len() == input.len()
                    || prefix.len() + input[prefix.len()..].chars().next().unwrap().len_utf8()
                        > budget
            );
        }
    }
}

#[test]
fn in_place_truncation_agrees_with_borrowed_prefix_at_every_boundary() {
    let input = "é茶🙂\nhello\0";
    for budget in 0..=input.len() + 1 {
        let mut owned = input.to_owned();
        truncate_utf8(&mut owned, budget);
        assert_eq!(owned, utf8_prefix(input, budget));
    }
}

#[test]
fn unlimited_budget_preserves_text_and_allocation() {
    let mut text = "λ🙂".to_owned();
    let pointer = text.as_ptr();
    truncate_utf8(&mut text, usize::MAX);
    assert_eq!(text, "λ🙂");
    assert_eq!(text.as_ptr(), pointer);
}

#[test]
fn bounded_append_accepts_an_exact_fit() {
    let mut text = "a".to_owned();
    assert!(push_bounded(&mut text, "茶", 4));
    assert_eq!(text, "a茶");
}

#[test]
fn bounded_append_never_splits_a_multibyte_character() {
    for budget in 1..=10 {
        let mut text = "a".to_owned();
        let complete = push_bounded(&mut text, "λ茶🙂", budget);
        assert_eq!(text, format!("a{}", utf8_prefix("λ茶🙂", budget - 1)));
        assert_eq!(complete, budget >= 10);
    }
}

#[test]
fn append_at_full_budget_rejects_nonempty_and_accepts_empty() {
    let mut text = "full".to_owned();
    assert!(!push_bounded(&mut text, "x", 4));
    assert!(push_bounded(&mut text, "", 4));
    assert_eq!(text, "full");
}

#[test]
fn append_with_an_already_over_budget_prefix_does_not_grow_it() {
    let mut text = "existing".to_owned();
    assert!(!push_bounded(&mut text, "more", 2));
    assert_eq!(text, "existing");
}

#[test]
fn marker_replaces_only_the_tail_needed_for_its_budget() {
    let mut text = "abcdef".to_owned();
    append_truncation_marker(&mut text, "...", 7);
    assert_eq!(text, "abcd...");
}

#[test]
fn marker_does_not_split_the_last_retained_character() {
    let mut text = "a🙂z".to_owned();
    append_truncation_marker(&mut text, "!", 4);
    assert_eq!(text, "a!");
}

#[test]
fn marker_can_fill_the_entire_budget() {
    let mut text = "old output".to_owned();
    append_truncation_marker(&mut text, "茶", 3);
    assert_eq!(text, "茶");
}

#[test]
fn oversized_marker_leaves_existing_text_unchanged() {
    let mut text = "old".to_owned();
    append_truncation_marker(&mut text, "too long", 3);
    assert_eq!(text, "old");
}

#[test]
fn ansi_free_unicode_whitespace_and_controls_are_not_modified() {
    let text = "λ🙂\r\n\t\0\x07 backspace:\x08";
    assert_eq!(strip_ansi(text), text);
}

#[test]
fn ansi_csi_punctuation_final_bytes_end_the_escape() {
    for final_byte in 0x40..=0x7e {
        assert!(is_csi_final_byte(final_byte));
        assert_eq!(
            strip_ansi(&format!("a\x1b[12{}visible", char::from(final_byte))),
            "avisible"
        );
    }
    for byte in [0, 0x1b, 0x3f, 0x7f, 0xff] {
        assert!(!is_csi_final_byte(byte));
    }
}

#[test]
fn ansi_osc_hyperlink_keeps_only_the_link_label() {
    assert_eq!(
        strip_ansi("\x1b]8;;https://example.org\x1b\\label\x1b]8;;\x1b\\"),
        "label"
    );
}

#[test]
fn ansi_osc_title_accepts_bell_and_string_terminators() {
    assert_eq!(strip_ansi("a\x1b]0;title\x07b"), "ab");
    assert_eq!(strip_ansi("a\x1b]0;title\x1b\\b"), "ab");
}

#[test]
fn ansi_control_strings_discard_embedded_escape_and_bell_bytes() {
    for kind in ['P', 'X', '^', '_'] {
        assert_eq!(
            strip_ansi(&format!("a\x1b{kind}data\x07more\x1bZstill\x1b\\b")),
            "ab"
        );
    }
}

#[test]
fn unterminated_escape_sequences_are_discarded_without_panicking() {
    for suffix in ["\x1b", "\x1b[123;", "\x1b]title", "\x1bPdata", "\x1b("] {
        assert_eq!(strip_ansi(&format!("prefix{suffix}")), "prefix");
    }
}

#[test]
fn charset_designations_and_cursor_sequences_preserve_following_text() {
    assert_eq!(strip_ansi("\x1b(0qqq\x1b(Btext\x1b7end"), "qqqtextend");
}

#[test]
fn stripping_already_stripped_output_is_idempotent() {
    let text = "\x1b[31mred\x1b[m\x1b]title\x07 λ\x1bPdata\x1b\\!";
    let plain = strip_ansi(text);
    assert_eq!(strip_ansi(&plain), plain);
}
