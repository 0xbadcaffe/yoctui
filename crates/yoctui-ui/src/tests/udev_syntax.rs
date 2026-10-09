use super::*;
use yoctui_model::Effect;

#[test]
fn udev_syntax_preserves_text_quotes_unicode_and_distinguishes_tokens() {
    let app = App::new(32, 8192);
    let content = "# Board rules\nACTION==\"add\", ENV{MODEL}!=i\"Écran#1\", MODE:=\"0660\", TAG+=\"systemd\" # note\nRUN+=e\"/bin/tool \\\"quoted\\\"\"";
    let text = source_render::source_preview(content, "99-board.rules", &app);
    let palette = ThemePalette::for_app(&app);
    assert_eq!(
        text.lines
            .iter()
            .map(|line| line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>())
            .collect::<Vec<_>>()
            .join("\n"),
        content
    );
    for (token, color) in [
        ("ACTION", palette.syntax_name),
        ("ENV{MODEL}", palette.syntax_name),
        ("==", palette.syntax_operator),
        (":=", palette.syntax_operator),
        ("i\"Écran#1\"", palette.syntax_value),
        ("# note", palette.syntax_comment),
    ] {
        assert!(
            text.lines
                .iter()
                .flat_map(|line| &line.spans)
                .any(|span| span.content == token && span.style.fg == Some(color)),
            "missing styled {token}"
        );
    }
    assert_eq!(
        yoctui_model::SourceLanguage::from_path(Path::new("99-board.rules")),
        yoctui_model::SourceLanguage::Udev
    );
}

#[test]
fn udev_syntax_is_used_in_rootfs_preview_without_changing_rules() {
    let app = readme_udev_rules_app();
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| rootfs_udev_workspace(frame, &app, frame.area()))
            .unwrap();
        let palette = ThemePalette::for_app(&app);
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|cell| cell.fg == palette.syntax_operator),
            "udev preview lacks colored operators at {width}x{height}"
        );
    }
}

#[test]
fn udev_syntax_rule_open_keeps_images_tab_and_masks_cannot_be_edited() {
    let mut app = readme_udev_rules_app();
    let Some(Effect::OpenLayerBrowserEditor { layer, root, file }) =
        update(&mut app, Action::EditSelectedRootfsSystemFile)
    else {
        panic!("selected udev file should open in the source editor");
    };
    assert_eq!(
        file,
        PathBuf::from("etc/udev/rules.d/60-persistent-serial.rules")
    );
    update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: layer,
            root,
            files: vec![file],
        },
    );
    update(
        &mut app,
        Action::LoadRecipeEditorContent("SUBSYSTEM==\"tty\", MODE=\"0660\"\n".into()),
    );
    let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
        panic!("editor missing");
    };
    assert_eq!(editor.context, yoctui_model::SourceEditorContext::Rootfs);
    assert_eq!(editor.focus, yoctui_model::RecipeEditorFocus::Document);
    assert_eq!(editor.document.position().line, 0);
    assert_eq!(app.images_view, ImagesView::UdevRules);
    assert!(rendered_text(&app, 160, 50).contains("Image file editor:"));
    update(&mut app, Action::CloseRecipeEditor);
    app.rootfs_udev_selection = 3;
    assert!(update(&mut app, Action::EditSelectedRootfsSystemFile).is_none());
}
