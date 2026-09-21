use super::*;

#[test]
fn ux_checkbox_batch_is_bounded_and_preview_never_executes() {
    let mut batch = CheckboxBatch::new(
        (0..CHECKBOX_MAX_BATCH_ROWS + 20)
            .map(|index| CheckboxState::new(format!("pkg:{index}"), format!("Package {index}"))),
    );
    assert_eq!(batch.rows().len(), CHECKBOX_MAX_BATCH_ROWS);
    assert!(batch.toggle_focused());
    batch.move_cursor(1);
    assert!(batch.toggle_focused());
    let preview = batch.preview(true);
    assert_eq!(preview.targets, ["pkg:0", "pkg:1"]);
    assert!(preview.destructive);
}
