use yoctui_model::*;

fn diagnostic() -> TextAreaValidationSpan {
    TextAreaValidationSpan {
        start: 0,
        end: 1,
        severity: TextAreaValidationSeverity::Error,
        message: "stale".into(),
    }
}

#[test]
fn constructor_truncates_on_a_utf8_boundary_at_the_document_limit() {
    let editor = TextAreaState::new("茶".repeat(TEXTAREA_MAX_BYTES / 3 + 1));
    assert_eq!(editor.text.len(), TEXTAREA_MAX_BYTES - 1);
    assert_eq!(editor.cursor, editor.text.len());
    assert!(!editor.is_modified());
}

#[test]
fn empty_insert_is_a_true_noop() {
    let mut editor = TextAreaState::new("text".into());
    let before = editor.clone();
    editor.try_insert("").unwrap();
    assert_eq!(editor, before);
}

#[test]
fn empty_insert_deletes_a_nonempty_selection() {
    let mut editor = TextAreaState::new("a茶b".into());
    editor.select_range(1, 4);
    editor.try_insert("").unwrap();
    assert_eq!(editor.text, "ab");
    assert_eq!(editor.cursor, 1);
    assert!(editor.undo());
    assert_eq!(editor.text, "a茶b");
}

#[test]
fn selection_clamps_reversed_out_of_range_and_mid_character_offsets() {
    let mut editor = TextAreaState::new("a茶🙂z".into());
    editor.select_range(usize::MAX, 2);
    assert_eq!(editor.selected_text(), Some("茶🙂z"));
    assert_eq!(editor.cursor, 1);
    assert_eq!(editor.selection, Some((1, 9)));
}

#[test]
fn backspace_and_delete_remove_whole_utf8_characters() {
    let mut editor = TextAreaState::new("a茶🙂".into());
    editor.backspace();
    assert_eq!(editor.text, "a茶");
    editor.move_cursor(TextAreaMotion::DocumentStart);
    editor.right();
    editor.delete_forward();
    assert_eq!(editor.text, "a");
}

#[test]
fn deletion_at_document_boundaries_is_a_noop() {
    let mut editor = TextAreaState::new("text".into());
    let before = editor.clone();
    editor.delete_forward();
    assert_eq!(editor, before);
    editor.move_cursor(TextAreaMotion::DocumentStart);
    let before = editor.clone();
    editor.backspace();
    assert_eq!(editor, before);
}

#[test]
fn clipboard_paste_replaces_selection_and_can_be_undone() {
    let mut editor = TextAreaState::new("before".into());
    editor.set_clipboard("茶\n🙂".into());
    editor.select_range(0, usize::MAX);
    editor.paste_internal_clipboard().unwrap();
    assert_eq!(editor.text, "茶\n🙂");
    assert!(editor.undo());
    assert_eq!(editor.text, "before");
}

#[test]
fn both_paste_sources_reject_oversized_input_without_mutation() {
    for source in [
        TextAreaPasteSource::Clipboard,
        TextAreaPasteSource::BracketedPaste,
    ] {
        let mut editor = TextAreaState::new("text".into());
        editor.select_range(0, 2);
        let before = editor.clone();
        assert!(matches!(
            editor.paste_text(&"x".repeat(TEXTAREA_MAX_PASTE_BYTES + 1), source),
            Err(TextAreaError::PasteLimit { .. })
        ));
        assert_eq!(editor, before);
    }
}

#[test]
fn internal_clipboard_truncates_at_a_utf8_boundary() {
    let mut editor = TextAreaState::new(String::new());
    editor.set_clipboard("茶".repeat(TEXTAREA_MAX_PASTE_BYTES / 3 + 1));
    editor.paste_internal_clipboard().unwrap();
    assert_eq!(editor.text.len(), TEXTAREA_MAX_PASTE_BYTES - 1);
}

#[test]
fn full_document_allows_equal_size_replacement_but_not_growth() {
    let mut editor = TextAreaState::new("x".repeat(TEXTAREA_MAX_BYTES));
    editor.select_range(0, 1);
    editor.try_insert("y").unwrap();
    assert_eq!(editor.text.len(), TEXTAREA_MAX_BYTES);
    let before = editor.clone();
    assert!(matches!(
        editor.try_insert("z"),
        Err(TextAreaError::TextLimit { .. })
    ));
    assert_eq!(editor, before);
}

#[test]
fn undo_and_redo_are_bounded_and_new_edits_discard_redo() {
    let mut editor = TextAreaState::new(String::new());
    for _ in 0..TEXTAREA_MAX_HISTORY + 10 {
        editor.insert("x");
    }
    assert_eq!(editor.history_lengths(), (TEXTAREA_MAX_HISTORY, 0));
    for _ in 0..TEXTAREA_MAX_HISTORY {
        assert!(editor.undo());
    }
    assert!(!editor.undo());
    assert_eq!(editor.text, "x".repeat(10));
    assert_eq!(editor.history_lengths(), (0, TEXTAREA_MAX_HISTORY));
    editor.insert("z");
    assert!(!editor.redo());
}

#[test]
fn redo_discards_validation_from_the_undone_revision() {
    let mut editor = TextAreaState::new("a".into());
    editor.insert("b");
    editor.undo();
    editor.set_validation([diagnostic()]);
    assert!(editor.redo());
    assert!(editor.validation().is_empty());
}

#[test]
fn external_saved_edit_discards_validation_and_remains_visually_diffable() {
    let mut editor = TextAreaState::new("a".into());
    editor.set_validation([diagnostic()]);
    editor.accept_external_edit("茶".into());
    assert!(editor.validation().is_empty());
    assert!(!editor.is_modified());
    assert!(editor.has_visual_diff());
}

#[test]
fn visual_selection_extends_then_clears_when_leaving_visual_mode() {
    let mut editor = TextAreaState::new("a茶b".into());
    editor.move_cursor(TextAreaMotion::DocumentStart);
    editor.set_mode(TextAreaMode::Visual);
    editor.right();
    editor.right();
    assert_eq!(editor.selected_text(), Some("a茶"));
    editor.set_mode(TextAreaMode::Normal);
    assert!(editor.selection.is_none());
}

#[test]
fn vertical_movement_retains_preferred_character_column_across_short_lines() {
    let mut editor = TextAreaState::new("abcdef\nx\n茶茶茶茶茶茶".into());
    editor.select_position(0, 5, false);
    editor.down();
    assert_eq!(editor.position(), TextAreaPosition { line: 1, column: 1 });
    editor.down();
    assert_eq!(editor.position(), TextAreaPosition { line: 2, column: 5 });
    editor.up();
    editor.up();
    assert_eq!(editor.position(), TextAreaPosition { line: 0, column: 5 });
}

#[test]
fn selecting_positions_clamps_to_document_and_line_ends() {
    let mut editor = TextAreaState::new("a\n茶\n".into());
    assert_eq!(editor.line_count(), 3);
    editor.select_position(1, usize::MAX, false);
    assert_eq!(editor.position(), TextAreaPosition { line: 1, column: 1 });
    editor.select_position(usize::MAX, usize::MAX, false);
    assert_eq!(editor.cursor, editor.text.len());
    assert_eq!(editor.position(), TextAreaPosition { line: 2, column: 0 });
}

#[test]
fn copying_a_line_or_selection_does_not_modify_text_or_history() {
    let mut editor = TextAreaState::new("first\n茶 tea\nlast".into());
    editor.select_position(1, 1, false);
    assert_eq!(editor.copy_selection_or_line(), "茶 tea");
    editor.select_range(6, 9);
    assert_eq!(editor.copy_selection_or_line(), "茶");
    assert!(!editor.is_modified());
    assert_eq!(editor.history_lengths(), (0, 0));
}

#[test]
fn validation_spans_are_bounded_ordered_and_utf8_safe() {
    let mut editor = TextAreaState::new("a茶🙂".into());
    editor.set_validation(std::iter::repeat_n(
        TextAreaValidationSpan {
            start: usize::MAX,
            end: 2,
            severity: TextAreaValidationSeverity::Warning,
            message: "茶".repeat(2000),
        },
        TEXTAREA_MAX_VALIDATION_SPANS + 1,
    ));
    assert_eq!(editor.validation().len(), TEXTAREA_MAX_VALIDATION_SPANS);
    for span in editor.validation() {
        assert_eq!((span.start, span.end), (1, 8));
        assert_eq!(span.message.len(), 4095);
    }
}

#[test]
fn wrapped_visual_lines_reconstruct_unicode_source_and_trailing_empty_line() {
    let mut editor = TextAreaState::new("a茶🙂z\n".into());
    editor.layout_mut().wrap_width = Some(2);
    let lines = editor.visual_lines(0, 10);
    assert_eq!(
        lines
            .iter()
            .map(|line| &editor.text[line.start..line.end])
            .collect::<Vec<_>>(),
        ["a茶", "🙂z", ""]
    );
    assert!(!lines[0].continuation);
    assert!(lines[1].continuation);
    assert_eq!(lines[2].source_line, 1);
}

#[test]
fn visual_projection_handles_zero_limits_zero_wrap_and_out_of_range_scroll() {
    let mut editor = TextAreaState::new("one\ntwo".into());
    assert!(editor.visual_lines(0, 0).is_empty());
    assert!(editor.visual_lines(usize::MAX, 10).is_empty());
    editor.layout_mut().wrap_width = Some(0);
    assert_eq!(editor.visual_lines(0, 10).len(), 2);
}
