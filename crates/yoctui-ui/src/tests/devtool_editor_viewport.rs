use super::*;

#[test]
fn devtool_editor_viewport_renders_selected_tail_long_document_and_distinct_panes() {
    let mut app = App::new(32, 8_192);
    let files = (0..40)
        .map(|index| PathBuf::from(format!("src/file-{index:02}.rs")))
        .collect::<Vec<_>>();
    let _ = update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "demo".into(),
            root: "/workspace/demo".into(),
            files,
        },
    );
    let _ = update(&mut app, Action::SelectRecipeEditorFile { delta: 35 });
    let _ = update(
        &mut app,
        Action::LoadRecipeEditorContent(
            (0..80)
                .map(|line| format!("let line_{line} = {line};"))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    );
    let _ = update(
        &mut app,
        Action::FocusRecipeEditor(yoctui_model::RecipeEditorFocus::Document),
    );
    for _ in 0..60 {
        let _ = update(
            &mut app,
            Action::EditRecipeEditor(yoctui_model::PopupEditorCommand::Down),
        );
    }
    let output = rendered_text(&app, 160, 50);
    for anchor in [
        "Files 36/40",
        "36/40",
        "file-35.rs",
        "let line_60",
        "Recipe Inspector",
        "Validation and diff state",
        "NORMAL",
    ] {
        assert!(output.contains(anchor), "missing {anchor:?}: {output}");
    }
    assert!(
        !output.contains("file-00.rs"),
        "tree did not follow selection"
    );
    assert!(
        !output.contains("let line_0 = 0;"),
        "document did not scroll"
    );
}

#[test]
fn devtool_editor_viewport_marks_a_limited_inventory_and_known_extensions() {
    let mut app = App::new(32, 8_192);
    app.dialogs.push_back(Dialog::RecipeEditor(RecipeEditor {
        recipe: "demo".into(),
        root: "/workspace/demo".into(),
        files: vec!["src/main.rs".into(), "config/settings.toml".into()],
        file_inventory_truncated: true,
        selection: 0,
        focus: yoctui_model::RecipeEditorFocus::Files,
        language: yoctui_model::SourceLanguage::Rust,
        document: yoctui_model::TextAreaState::new("fn main() {}".into()),
        searching: false,
    }));
    let mut terminal = Terminal::new(TestBackend::new(160, 50)).unwrap();
    terminal.draw(|frame| render(frame, &app)).unwrap();
    let buffer = terminal.backend().buffer();
    let text = buffer
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(text.contains("limited"), "{text}");
    let palette = ThemePalette::for_app(&app);
    assert!(
        buffer
            .content
            .iter()
            .any(|cell| cell.symbol() == "." && cell.fg == palette.syntax_keyword),
        "known Rust extension was not highlighted"
    );
}
