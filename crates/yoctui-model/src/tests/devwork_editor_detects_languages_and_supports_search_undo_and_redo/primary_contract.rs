use super::*;

#[test]
fn devwork_editor_detects_languages_and_supports_search_undo_and_redo() {
    assert_eq!(
        SourceLanguage::from_path(Path::new("recipe.bbappend")),
        SourceLanguage::BitBake
    );
    assert_eq!(
        SourceLanguage::from_path(Path::new("src/main.cpp")),
        SourceLanguage::Cpp
    );
    assert_eq!(
        SourceLanguage::from_path(Path::new("src/lib.rs")),
        SourceLanguage::Rust
    );
    assert_eq!(
        SourceLanguage::from_path(Path::new("Makefile")),
        SourceLanguage::Make
    );
    assert_eq!(
        SourceLanguage::from_path(Path::new("arch/arm/boot/dts/board.dts")),
        SourceLanguage::DeviceTree
    );
    assert_eq!(
        SourceLanguage::from_path(Path::new("soc/common.dtsi")),
        SourceLanguage::DeviceTree
    );

    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "demo".into(),
            root: "/workspace/demo".into(),
            files: vec!["src/main.cpp".into()],
        },
    );
    let _ = update(
        &mut app,
        Action::LoadRecipeEditorContent("int main() { return 0; }".into()),
    );
    let _ = update(
        &mut app,
        Action::FocusRecipeEditor(RecipeEditorFocus::Document),
    );
    let _ = update(
        &mut app,
        Action::EditRecipeEditor(PopupEditorCommand::ToggleInsert),
    );
    let _ = update(
        &mut app,
        Action::EditRecipeEditor(PopupEditorCommand::Newline),
    );
    let _ = update(&mut app, Action::EditRecipeEditor(PopupEditorCommand::Undo));
    let _ = update(&mut app, Action::EditRecipeEditor(PopupEditorCommand::Redo));
    let _ = update(&mut app, Action::BeginRecipeEditorSearch);
    for character in "return".chars() {
        let _ = update(&mut app, Action::AppendRecipeEditorSearch(character));
    }
    let editor = match app.active_dialog() {
        Some(Dialog::RecipeEditor(editor)) => editor,
        other => panic!("expected recipe editor, got {other:?}"),
    };
    assert_eq!(editor.language, SourceLanguage::Cpp);
    assert_eq!(editor.document.search_state().matches.len(), 1);
    assert!(editor.is_dirty());
}
