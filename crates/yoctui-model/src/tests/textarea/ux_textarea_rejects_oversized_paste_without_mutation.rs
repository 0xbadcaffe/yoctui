use super::*;

#[test]
fn ux_textarea_rejects_oversized_paste_without_mutation() {
    let mut editor = TextAreaState::new("safe".into());
    let before = editor.clone();
    let error = editor.paste_text(
        &"x".repeat(TEXTAREA_MAX_PASTE_BYTES + 1),
        TextAreaPasteSource::BracketedPaste,
    );
    assert_eq!(
        error,
        Err(TextAreaError::PasteLimit {
            limit: TEXTAREA_MAX_PASTE_BYTES
        })
    );
    assert_eq!(editor, before);
}
