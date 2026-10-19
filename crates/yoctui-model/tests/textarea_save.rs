use yoctui_model::*;

fn modified() -> TextAreaState {
    let mut editor = TextAreaState::new("old\n".into());
    editor.select_range(0, usize::MAX);
    editor.insert("new\n");
    editor
}

#[test]
fn revision_distinguishes_same_length_content_and_counts_utf8_bytes() {
    assert_ne!(TextAreaRevision::of("a"), TextAreaRevision::of("b"));
    assert_eq!(TextAreaRevision::of("茶").bytes, 3);
    assert_eq!(TextAreaRevision::of("same"), TextAreaRevision::of("same"));
}

#[test]
fn save_request_captures_content_revision_and_sibling_temporary_path() {
    let mut editor = modified();
    let request = editor
        .begin_atomic_save("/tmp/local.conf", TextAreaRevision::of("old\n"))
        .unwrap();
    assert_eq!(request.content, "new\n");
    assert_eq!(request.revision, TextAreaRevision::of(&request.content));
    assert_eq!(request.temporary.parent(), request.target.parent());
    assert_ne!(request.temporary, request.target);
    assert!(matches!(
        editor.save_state(),
        TextAreaSaveState::Saving { .. }
    ));
}

#[test]
fn save_conflict_preserves_text_selection_and_history() {
    let mut editor = modified();
    let text = editor.text.clone();
    let history = editor.history_lengths();
    let selection = editor.selection;
    let cursor = editor.cursor;
    let observed = TextAreaRevision::of("edited outside");
    assert!(
        editor
            .begin_atomic_save("/tmp/local.conf", observed)
            .is_none()
    );
    assert_eq!(editor.text, text);
    assert_eq!(editor.history_lengths(), history);
    assert_eq!(editor.selection, selection);
    assert_eq!(editor.cursor, cursor);
    assert!(
        matches!(editor.save_state(), TextAreaSaveState::Conflict { expected, observed: actual, .. }
        if *expected == TextAreaRevision::of("old\n") && *actual == observed)
    );
}

#[test]
fn matching_save_acknowledgement_advances_both_baselines() {
    let mut editor = modified();
    let request = editor
        .begin_atomic_save("/tmp/config", editor.base_revision())
        .unwrap();
    assert!(editor.mark_saved(&request));
    assert!(!editor.is_modified());
    assert!(!editor.has_visual_diff());
    assert_eq!(editor.base_revision(), request.revision);
    assert!(matches!(
        editor.save_state(),
        TextAreaSaveState::Saved { .. }
    ));
}

#[test]
fn duplicate_save_acknowledgements_do_not_mutate_state() {
    let mut editor = modified();
    let request = editor
        .begin_atomic_save("/tmp/config", editor.base_revision())
        .unwrap();
    assert!(editor.mark_saved(&request));
    let before = editor.clone();
    assert!(!editor.mark_saved(&request));
    assert_eq!(editor, before);
}

#[test]
fn stale_save_acknowledgement_cannot_mark_a_newer_edit_clean() {
    let mut editor = modified();
    let request = editor
        .begin_atomic_save("/tmp/config", editor.base_revision())
        .unwrap();
    editor.insert("later");
    let before = editor.clone();
    assert!(!editor.mark_saved(&request));
    assert_eq!(editor, before);
    assert!(editor.is_modified());
}

#[test]
fn save_acknowledgement_for_a_different_target_is_rejected() {
    let mut editor = modified();
    let mut request = editor
        .begin_atomic_save("/tmp/config", editor.base_revision())
        .unwrap();
    request.target = "/tmp/other".into();
    let before = editor.clone();
    assert!(!editor.mark_saved(&request));
    assert_eq!(editor, before);
}

#[test]
fn save_acknowledgement_for_a_different_temporary_path_is_rejected() {
    let mut editor = modified();
    let mut request = editor
        .begin_atomic_save("/tmp/config", editor.base_revision())
        .unwrap();
    request.temporary = "/tmp/wrong-temp".into();
    assert!(!editor.mark_saved(&request));
    assert!(editor.is_modified());
}

#[test]
fn save_acknowledgement_for_a_different_revision_is_rejected() {
    let mut editor = modified();
    let mut request = editor
        .begin_atomic_save("/tmp/config", editor.base_revision())
        .unwrap();
    request.revision = TextAreaRevision::of("wrong");
    assert!(!editor.mark_saved(&request));
    assert!(editor.is_modified());
}

#[test]
fn recoverable_failure_retry_preserves_the_exact_request() {
    let mut editor = modified();
    let request = editor
        .begin_atomic_save("/tmp/config", editor.base_revision())
        .unwrap();
    assert!(editor.mark_save_failed("disk full", true));
    assert_eq!(editor.retry_save().unwrap(), request);
    assert!(editor.mark_saved(&request));
}

#[test]
fn nonrecoverable_failure_cannot_be_retried() {
    let mut editor = modified();
    editor
        .begin_atomic_save("/tmp/config", editor.base_revision())
        .unwrap();
    assert!(editor.mark_save_failed("permission denied", false));
    let before = editor.clone();
    assert_eq!(editor.retry_save(), Err(TextAreaError::NoSaveFailure));
    assert_eq!(editor, before);
}

#[test]
fn retry_and_failure_without_a_pending_save_are_noops() {
    let mut editor = modified();
    let before = editor.clone();
    assert_eq!(editor.retry_save(), Err(TextAreaError::NoSaveFailure));
    assert!(!editor.mark_save_failed("nothing pending", true));
    assert_eq!(editor, before);
}

#[test]
fn save_failure_messages_are_bounded_without_splitting_utf8() {
    let mut editor = modified();
    editor
        .begin_atomic_save("/tmp/config", editor.base_revision())
        .unwrap();
    editor.mark_save_failed("茶".repeat(3000), true);
    let TextAreaSaveState::Failed { message, .. } = editor.save_state() else {
        panic!("expected failure")
    };
    assert_eq!(message.len(), 8190);
    assert!(message.chars().all(|c| c == '茶'));
}

#[test]
fn read_only_diff_rows_do_not_change_save_state_or_history() {
    let editor = modified();
    let before = editor.clone();
    let rows = editor.changed_lines(10);
    assert_eq!(
        rows.iter()
            .map(|line| (line.kind, line.text.as_str()))
            .collect::<Vec<_>>(),
        [
            (TextAreaDiffKind::Removed, "old"),
            (TextAreaDiffKind::Added, "new")
        ]
    );
    assert_eq!(editor, before);
    assert!(editor.changed_lines(0).is_empty());
    assert_eq!(editor.changed_lines(1).len(), 1);
}

#[test]
fn successful_save_allows_undo_and_redo_without_losing_the_saved_baseline() {
    let mut editor = modified();
    let request = editor
        .begin_atomic_save("/tmp/config", editor.base_revision())
        .unwrap();
    assert!(editor.mark_saved(&request));
    assert!(editor.undo());
    assert_eq!(editor.text, "old\n");
    assert!(editor.is_modified());
    assert!(editor.redo());
    assert_eq!(editor.text, "new\n");
    assert!(!editor.is_modified());
}

#[test]
fn accepting_new_external_content_resets_old_selection_search_and_history() {
    let mut editor = modified();
    editor.search("new", true).unwrap();
    editor.accept_external_text("loaded".into());
    assert_eq!(editor.text, "loaded");
    assert_eq!(editor.cursor, 6);
    assert!(editor.selection.is_none());
    assert!(editor.search_state().query.is_empty());
    assert_eq!(editor.history_lengths(), (0, 0));
    assert!(!editor.is_modified());
    assert!(!editor.has_visual_diff());
}
