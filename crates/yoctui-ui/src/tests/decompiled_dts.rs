use super::*;

#[test]
fn decompiled_dts_has_correct_title_document_viewport_and_tail_at_responsive_sizes() {
    let mut app = App::new(32, 8192);
    update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "Image decompiled DTS".into(),
            root: "/tmp/board".into(),
            files: vec!["board.dts".into()],
        },
    );
    if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
        editor.context = yoctui_model::SourceEditorContext::DeviceTree;
        editor.focus = yoctui_model::RecipeEditorFocus::Document;
    }
    update(
        &mut app,
        Action::LoadRecipeEditorContent(
            (0..100)
                .map(|i| format!("line_{i:03} = <{i}>;"))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    );
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("Device tree editor:"), "{text}");
        assert!(text.contains("line_000"), "{text}");
        assert!(!text.contains("Recipe Inspector"));
    }
    update(
        &mut app,
        Action::EditRecipeEditor(yoctui_model::PopupEditorCommand::PageDown),
    );
    let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
        panic!("editor missing");
    };
    assert!(editor.document.position().line > 0);
    update(
        &mut app,
        Action::EditRecipeEditor(yoctui_model::PopupEditorCommand::SelectPosition {
            line: usize::MAX,
            column: 0,
            extend: false,
        }),
    );
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        assert!(rendered_text(&app, width, height).contains("line_099"));
    }
}
