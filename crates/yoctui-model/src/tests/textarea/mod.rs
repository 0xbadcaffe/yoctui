use super::*;

mod dirty_checks_follow_exact_text_and_independent_baselines;

mod ux_textarea_unicode_motion_selection_and_bounded_history;

mod ux_textarea_search_replace_undo_redo_remain_utf8_safe;

mod ux_textarea_layout_projects_line_numbers_and_wrap_metadata;

mod ux_textarea_validation_diff_conflict_and_recoverable_atomic_save_are_typed;

mod ux_textarea_page_word_and_line_motion_use_character_columns;

mod ux_textarea_rejects_oversized_paste_without_mutation;

mod external_saved_edit_remains_diffable_without_blocking_the_build;

#[cfg(feature = "proptest")]
mod properties {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn ux_textarea_adversarial_sequences_keep_utf8_boundaries(
            initial in ".{0,512}",
            operations in prop::collection::vec((0u8..12, ".{0,16}"), 0..400)
        ) {
            let mut editor = TextAreaState::new(initial);
            for (operation, value) in operations {
                match operation {
                    0 => { let _ = editor.try_insert(&value); }
                    1 => editor.backspace(),
                    2 => editor.delete_forward(),
                    3 => editor.move_cursor(TextAreaMotion::Left),
                    4 => editor.move_cursor(TextAreaMotion::Right),
                    5 => editor.move_cursor(TextAreaMotion::Up),
                    6 => editor.move_cursor(TextAreaMotion::Down),
                    7 => { editor.undo(); }
                    8 => { editor.redo(); }
                    9 => { editor.set_mode(TextAreaMode::Visual); editor.move_cursor(TextAreaMotion::WordRight); }
                    10 => { let _ = editor.search(value, true); }
                    _ => { let _ = editor.replace_all(&value); }
                }
                prop_assert!(editor.text.len() <= TEXTAREA_MAX_BYTES);
                prop_assert!(editor.text.is_char_boundary(editor.cursor));
                if let Some((start, end)) = editor.selection {
                    prop_assert!(start <= end && end <= editor.text.len());
                    prop_assert!(editor.text.is_char_boundary(start));
                    prop_assert!(editor.text.is_char_boundary(end));
                }
                let (undo, redo) = editor.history_lengths();
                prop_assert!(undo <= TEXTAREA_MAX_HISTORY);
                prop_assert!(redo <= TEXTAREA_MAX_HISTORY);
            }
        }
    }
}
