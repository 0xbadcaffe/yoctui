use super::*;

#[test]
fn devtool_editor_viewport_follows_every_retained_file_and_long_document_cursor() {
    let files = (0..40)
        .map(|index| PathBuf::from(format!("src/file-{index:02}.rs")))
        .collect::<Vec<_>>();
    let mut app = App::new(16, 4_096);
    let _ = update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "demo".into(),
            root: "/workspace/demo".into(),
            files,
        },
    );
    let _ = update(&mut app, Action::SelectRecipeEditorFile { delta: 31 });
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
        Action::FocusRecipeEditor(RecipeEditorFocus::Document),
    );
    for _ in 0..55 {
        let _ = update(&mut app, Action::EditRecipeEditor(PopupEditorCommand::Down));
    }
    let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
        panic!("recipe editor missing")
    };
    assert_eq!(editor.selection, 31);
    assert!(editor.file_viewport(7).contains(&31));
    let cursor_line = editor.document.position().line;
    assert!(cursor_line >= 50, "cursor stopped at line {cursor_line}");
    assert!(editor.document_viewport(9).contains(&cursor_line));
    assert_eq!(editor.language, SourceLanguage::Rust);
}

#[test]
fn devtool_editor_viewport_bounds_inventory_and_classifies_every_known_language() {
    let files = (0..=MAX_RECIPE_EDITOR_FILES)
        .map(|index| PathBuf::from(format!("file-{index}.c")))
        .collect();
    let mut app = App::new(16, 4_096);
    let _ = update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "large".into(),
            root: "/workspace/large".into(),
            files,
        },
    );
    let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
        panic!("recipe editor missing")
    };
    assert_eq!(editor.files.len(), MAX_RECIPE_EDITOR_FILES);
    assert!(editor.file_inventory_truncated);

    for (path, language) in [
        ("recipe.bbclass", SourceLanguage::BitBake),
        ("image.wks.in", SourceLanguage::BitBake),
        ("main.c", SourceLanguage::C),
        ("main.hpp", SourceLanguage::Cpp),
        ("lib.rs", SourceLanguage::Rust),
        ("tool.py", SourceLanguage::Python),
        ("run.sh", SourceLanguage::Shell),
        ("web.mjs", SourceLanguage::JavaScript),
        ("web.tsx", SourceLanguage::TypeScript),
        ("data.json", SourceLanguage::Json),
        ("Cargo.toml", SourceLanguage::Toml),
        ("config.yaml", SourceLanguage::Yaml),
        ("Makefile", SourceLanguage::Make),
        ("README.md", SourceLanguage::Markdown),
        ("board.dtsi", SourceLanguage::DeviceTree),
        ("COPYING", SourceLanguage::PlainText),
    ] {
        assert_eq!(
            SourceLanguage::from_path(Path::new(path)),
            language,
            "{path}"
        );
    }
}
