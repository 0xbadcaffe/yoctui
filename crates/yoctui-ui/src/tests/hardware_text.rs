use super::*;

#[test]
fn hardware_text_editor_has_highlighting_and_edit_save_hints_at_responsive_sizes() {
    let mut app = App::new(32, 4096);
    app.screen = Screen::Hardware;
    app.focus = FocusTarget::Dialog;
    app.dialogs.push_back(Dialog::RecipeEditor(RecipeEditor {
        recipe: "Hardware: driver.c".into(),
        root: "/data/board".into(),
        files: vec!["driver.c".into()],
        file_inventory_truncated: false,
        context: yoctui_model::SourceEditorContext::HardwareProject,
        selection: 0,
        focus: yoctui_model::RecipeEditorFocus::Document,
        language: SourceLanguage::C,
        document: yoctui_model::TextAreaState::new("int answer = 42;\n".into()),
        searching: false,
        pending_search_position: None,
    }));
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("Ctrl+S save"), "{text}");
        assert!(text.contains("Source editor:"), "{text}");
        assert!(!text.contains("build recipe"));
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let palette = ThemePalette::for_app(&app);
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .any(|cell| cell.symbol() == "i" && cell.fg == palette.syntax_keyword)
        );
    }
    for (width, height) in [(60, 15), (20, 5), (1, 1)] {
        let _ = rendered_text(&app, width, height);
    }
}
