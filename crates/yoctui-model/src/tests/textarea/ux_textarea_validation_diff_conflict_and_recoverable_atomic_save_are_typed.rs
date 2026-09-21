use super::*;

#[test]
fn ux_textarea_validation_diff_conflict_and_recoverable_atomic_save_are_typed() {
    let mut editor = TextAreaState::new("A=1\nB=2\n".into());
    editor.set_validation([TextAreaValidationSpan {
        start: 2,
        end: 3,
        severity: TextAreaValidationSeverity::Error,
        message: "invalid value".into(),
    }]);
    assert_eq!(editor.validation().len(), 1);
    editor.select_range(2, 3);
    editor.try_insert("3").unwrap();
    let preview = editor.preview_diff().clone();
    assert!(
        preview
            .lines
            .iter()
            .any(|line| line.kind == TextAreaDiffKind::Removed)
    );
    assert!(
        preview
            .lines
            .iter()
            .any(|line| line.kind == TextAreaDiffKind::Added)
    );

    let base = TextAreaRevision::of("A=1\nB=2\n");
    let request = editor.begin_atomic_save("/tmp/config", base).unwrap();
    assert_eq!(request.temporary.parent(), request.target.parent());
    assert!(editor.mark_save_failed("disk full", true));
    let retry = editor.retry_save().unwrap();
    assert_eq!(retry, request);
    assert!(editor.mark_saved(&retry));

    editor.try_insert("x").unwrap();
    assert!(editor.begin_atomic_save("/tmp/config", base).is_none());
    assert!(matches!(
        editor.save_state(),
        TextAreaSaveState::Conflict { .. }
    ));
}
