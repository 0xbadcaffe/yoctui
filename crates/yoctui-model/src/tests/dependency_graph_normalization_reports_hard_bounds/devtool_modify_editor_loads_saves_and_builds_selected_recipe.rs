use super::*;

#[test]
fn devtool_modify_editor_loads_saves_and_builds_selected_recipe() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        ..Recipe::default()
    });
    let root = PathBuf::from("/build/workspace/sources/busybox");
    assert_eq!(
        update(
            &mut app,
            Action::OpenRecipeEditor {
                recipe: "busybox".into(),
                root: root.clone(),
                files: vec![PathBuf::from("main.c")],
            },
        ),
        Some(Effect::LoadRecipeEditorFile(root.join("main.c")))
    );
    let _ = update(
        &mut app,
        Action::LoadRecipeEditorContent("int main() {}".into()),
    );
    let _ = update(&mut app, Action::ToggleRecipeEditorEditing);
    let _ = update(
        &mut app,
        Action::EditRecipeEditor(PopupEditorCommand::Newline),
    );
    let editor = match app.active_dialog() {
        Some(Dialog::RecipeEditor(editor)) => editor,
        other => panic!("expected recipe editor, got {other:?}"),
    };
    assert_eq!(
        editor.document.base_revision(),
        TextAreaRevision::of("int main() {}")
    );
    assert!(editor.is_dirty());
    assert!(editor.local_validation().is_empty());
    let _ = update(&mut app, Action::BeginRecipeEditorBuild);
    assert_eq!(
        app.notification.as_deref(),
        Some("Save workspace changes before starting the recipe build.")
    );
    assert!(matches!(app.active_dialog(), Some(Dialog::RecipeEditor(_))));
    assert_eq!(
        update(&mut app, Action::SaveRecipeEditor),
        Some(Effect::SaveRecipeEditorFile {
            root: root.clone(),
            path: root.join("main.c"),
            content: "int main() {}\n".into(),
            expected: TextAreaRevision::of("int main() {}"),
        })
    );
    let _ = update(&mut app, Action::RecipeEditorSaved);
    let editor = match app.active_dialog() {
        Some(Dialog::RecipeEditor(editor)) => editor,
        other => panic!("expected recipe editor, got {other:?}"),
    };
    assert!(!editor.document.is_modified());
    assert!(editor.diff_preview(2).is_empty());
    let _ = update(&mut app, Action::BeginRecipeEditorBuild);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
            targets: vec!["busybox".into()],
            task: None,
            force: false,
        }))
    );
}
