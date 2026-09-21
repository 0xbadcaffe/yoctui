use super::*;

#[test]
fn bbmask_editing_requires_a_preview_and_confirmation() {
    let mut app = App::new(10, 1_000);
    app.workspace
        .variables
        .insert("BBMASK".into(), "meta-old/.*".into());
    let _ = update(&mut app, Action::BeginBbmaskEdit);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::BbmaskEdit(editor))
            if editor.text == "bbmask = \"meta-old/.*\"\n"
                && editor.selected_text() == Some("meta-old/.*")
    ));
    if let Some(Dialog::BbmaskEdit(editor)) = app.active_dialog_mut() {
        editor.text = "bbmask = \"meta-old/.* x\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::PreviewBbmaskEdit);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::BbmaskConfirmation("meta-old/.* x".into()))
    );
    assert_eq!(
        update(&mut app, Action::ConfirmBbmaskWrite),
        Some(Effect::WriteBbmask("meta-old/.* x".into()))
    );
}
