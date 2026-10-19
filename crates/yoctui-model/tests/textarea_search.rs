use yoctui_model::{TEXTAREA_MAX_BYTES, TextAreaError, TextAreaRevision, TextAreaState};
use yoctui_model::{TEXTAREA_MAX_SEARCH_MATCHES, TextAreaMotion};

#[test]
fn case_insensitive_replacement_does_not_overlap_matches() {
    let mut editor = TextAreaState::new("aaaa".into());
    editor.search("aa", false).unwrap();
    assert_eq!(editor.search_state().matches, [(0, 2), (2, 4)]);
    assert_eq!(editor.replace_all("").unwrap(), 2);
    assert_eq!(editor.text, "");
}

#[test]
fn unicode_case_insensitive_replacement_preserves_character_boundaries() {
    let mut editor = TextAreaState::new("éééé".into());
    editor.search("ÉÉ", false).unwrap();
    assert_eq!(editor.replace_all("茶").unwrap(), 2);
    assert_eq!(editor.text, "茶茶");
}

#[test]
fn rejected_selected_replacement_is_fully_atomic() {
    let mut editor = TextAreaState::new("target".into());
    editor.search("target", true).unwrap();
    let before = editor.clone();
    assert_eq!(
        editor.replace_selected_match(&"x".repeat(TEXTAREA_MAX_BYTES + 1)),
        Err(TextAreaError::TextLimit {
            limit: TEXTAREA_MAX_BYTES
        })
    );
    assert_eq!(editor, before);
}

#[test]
fn undo_discards_validation_for_the_previous_revision() {
    let mut editor = TextAreaState::new("a".into());
    editor.insert("long");
    editor.set_validation([yoctui_model::TextAreaValidationSpan {
        start: 4,
        end: 5,
        severity: yoctui_model::TextAreaValidationSeverity::Error,
        message: "old revision".into(),
    }]);
    assert!(editor.undo());
    assert!(editor.validation().is_empty());
    assert_eq!(editor.base_revision(), TextAreaRevision::of("a"));
}

#[test]
fn search_is_literal_not_a_regular_expression() {
    let mut editor = TextAreaState::new("a.*b aZZb".into());
    editor.search(".*", true).unwrap();
    assert_eq!(editor.search_state().matches, [(1, 3)]);
}

#[test]
fn empty_search_clears_previous_results_and_reports_its_error() {
    let mut editor = TextAreaState::new("one one".into());
    editor.search("one", true).unwrap();
    assert_eq!(editor.search("", true), Err(TextAreaError::EmptySearch));
    assert!(editor.search_state().matches.is_empty());
    assert!(editor.search_state().query.is_empty());
    assert!(!editor.next_match(false));
}

#[test]
fn unmatched_search_has_no_selected_match_or_replacement() {
    let mut editor = TextAreaState::new("one".into());
    editor.search("two", false).unwrap();
    assert_eq!(editor.search_state().selected, None);
    assert!(!editor.next_match(true));
    assert!(!editor.replace_selected_match("x").unwrap());
    assert_eq!(editor.replace_all("x").unwrap(), 0);
    assert_eq!(editor.text, "one");
    assert_eq!(editor.history_lengths(), (0, 0));
}

#[test]
fn search_starts_at_cursor_then_wraps_to_the_beginning() {
    let mut editor = TextAreaState::new("one one one".into());
    editor.select_position(0, 4, false);
    editor.search("one", true).unwrap();
    assert_eq!(editor.search_state().selected, Some(1));
    assert!(editor.next_match(false));
    assert_eq!(editor.selected_text(), Some("one"));
    assert_eq!(editor.cursor, 11);
    assert!(editor.next_match(false));
    assert_eq!(editor.cursor, 3);
}

#[test]
fn previous_match_wraps_to_the_last_match() {
    let mut editor = TextAreaState::new("one one".into());
    editor.move_cursor(TextAreaMotion::DocumentStart);
    editor.search("one", true).unwrap();
    assert!(editor.next_match(true));
    assert_eq!(editor.search_state().selected, Some(1));
    assert_eq!(editor.cursor, 7);
}

#[test]
fn case_sensitive_search_distinguishes_ascii_and_unicode_case() {
    let mut editor = TextAreaState::new("café CAFÉ Café".into());
    editor.search("CAFÉ", true).unwrap();
    assert_eq!(editor.search_state().matches, [(6, 11)]);
}

#[test]
fn case_insensitive_search_keeps_offsets_in_original_utf8_bytes() {
    let mut editor = TextAreaState::new("café CAFÉ Café".into());
    editor.search("cAfÉ", false).unwrap();
    assert_eq!(editor.search_state().matches, [(0, 5), (6, 11), (12, 17)]);
}

#[test]
fn case_expanding_unicode_matches_keep_whole_original_characters() {
    let mut editor = TextAreaState::new("İ i\u{307}".into());
    editor.search("i\u{307}", false).unwrap();
    assert_eq!(editor.search_state().matches, [(0, 2), (3, 6)]);
    assert_eq!(editor.replace_all("x").unwrap(), 2);
    assert_eq!(editor.text, "x x");
}

#[test]
fn search_results_are_bounded_and_report_truncation_for_both_case_modes() {
    for case_sensitive in [false, true] {
        let mut editor = TextAreaState::new("a".repeat(TEXTAREA_MAX_SEARCH_MATCHES + 1));
        editor.search("a", case_sensitive).unwrap();
        assert_eq!(
            editor.search_state().matches.len(),
            TEXTAREA_MAX_SEARCH_MATCHES
        );
        assert!(editor.search_state().truncated);
        assert_eq!(
            editor.search_state().matches.last(),
            Some(&(TEXTAREA_MAX_SEARCH_MATCHES - 1, TEXTAREA_MAX_SEARCH_MATCHES))
        );
    }
}

#[test]
fn exactly_the_search_match_limit_is_not_marked_truncated() {
    for case_sensitive in [false, true] {
        let mut editor = TextAreaState::new("a".repeat(TEXTAREA_MAX_SEARCH_MATCHES));
        editor.search("a", case_sensitive).unwrap();
        assert_eq!(
            editor.search_state().matches.len(),
            TEXTAREA_MAX_SEARCH_MATCHES
        );
        assert!(!editor.search_state().truncated);
    }
}

#[test]
fn oversized_query_is_truncated_only_at_a_utf8_boundary() {
    let mut editor = TextAreaState::new("other".into());
    editor.search("茶".repeat(2000), true).unwrap();
    assert_eq!(editor.search_state().query.len(), 4095);
    assert!(editor.search_state().query.chars().all(|c| c == '茶'));
}

#[test]
fn replacement_text_is_not_recursively_searched_during_replace_all() {
    let mut editor = TextAreaState::new("a a".into());
    editor.search("a", true).unwrap();
    assert_eq!(editor.replace_all("aa").unwrap(), 2);
    assert_eq!(editor.text, "aa aa");
}

#[test]
fn replace_all_is_a_single_undoable_operation() {
    let mut editor = TextAreaState::new("one one one".into());
    editor.search("one", false).unwrap();
    editor.replace_all("茶").unwrap();
    assert_eq!(editor.text, "茶 茶 茶");
    assert_eq!(editor.history_lengths(), (1, 0));
    assert!(editor.undo());
    assert_eq!(editor.text, "one one one");
    assert!(editor.redo());
    assert_eq!(editor.text, "茶 茶 茶");
}

#[test]
fn rejected_replace_all_preserves_the_entire_editor_state() {
    let mut editor = TextAreaState::new("x x".into());
    editor.search("x", true).unwrap();
    let before = editor.clone();
    assert!(matches!(
        editor.replace_all(&"a".repeat(TEXTAREA_MAX_BYTES)),
        Err(TextAreaError::TextLimit { .. })
    ));
    assert_eq!(editor, before);
}

#[test]
fn replacing_a_match_selects_the_next_retained_result() {
    let mut editor = TextAreaState::new("a a a".into());
    editor.search("a", true).unwrap();
    assert!(editor.replace_selected_match("z").unwrap());
    assert_eq!(editor.text, "z a a");
    assert_eq!(editor.selected_text(), Some("a"));
    assert_eq!(editor.search_state().selected, Some(0));
}

#[test]
fn replace_all_clamps_cursor_when_replacement_shortens_multibyte_text() {
    let mut editor = TextAreaState::new("xx🙂".into());
    editor.search("x", true).unwrap();
    editor.move_cursor(TextAreaMotion::DocumentEnd);
    editor.replace_all("").unwrap();
    assert_eq!(editor.text, "🙂");
    assert!(editor.text.is_char_boundary(editor.cursor));
    assert!(editor.selection.is_none());
}

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn ascii_case_insensitive_replacement_matches_a_nonoverlapping_reference(
        source in "[aAbB ]{0,100}", query in "[ab]{1,4}", replacement in "[xX]{0,4}"
    ) {
        let lowered = source.to_ascii_lowercase();
        let ranges: Vec<_> = lowered.match_indices(&query).map(|(start, value)| (start, start + value.len())).collect();
        let mut expected = source.clone();
        for &(start, end) in ranges.iter().rev() { expected.replace_range(start..end, &replacement); }
        let mut editor = TextAreaState::new(source);
        editor.search(query, false).unwrap();
        prop_assert_eq!(&editor.search_state().matches, &ranges);
        prop_assert_eq!(editor.replace_all(&replacement).unwrap(), ranges.len());
        prop_assert_eq!(&editor.text, &expected);
        prop_assert!(editor.text.is_char_boundary(editor.cursor));
    }

    #[test]
    fn unicode_repeated_search_replacements_never_overlap_or_panic(
        repetitions in 0usize..100, query_length in 1usize..8,
        symbol in prop::sample::select(vec!["é", "茶", "🙂", "İ"])
    ) {
        let source = symbol.repeat(repetitions);
        let mut editor = TextAreaState::new(source);
        editor.search(symbol.repeat(query_length), false).unwrap();
        let count = repetitions / query_length;
        prop_assert_eq!(editor.replace_all("!").unwrap(), count);
        prop_assert_eq!(&editor.text, &format!("{}{}", "!".repeat(count), symbol.repeat(repetitions % query_length)));
    }
}
