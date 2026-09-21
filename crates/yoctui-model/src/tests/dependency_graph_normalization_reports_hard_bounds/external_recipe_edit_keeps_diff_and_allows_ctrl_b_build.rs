use super::*;

#[test]
fn external_recipe_edit_keeps_diff_and_allows_ctrl_b_build() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        ..Recipe::default()
    });
    let _ = update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "busybox".into(),
            root: "/workspace/busybox".into(),
            files: vec!["main.c".into()],
        },
    );
    let _ = update(
        &mut app,
        Action::LoadRecipeEditorContent("int value = 1;\n".into()),
    );
    let _ = update(
        &mut app,
        Action::LoadRecipeEditorExternalContent("int value = 2;\n".into()),
    );
    let editor = match app.active_dialog() {
        Some(Dialog::RecipeEditor(editor)) => editor,
        other => panic!("expected recipe editor, got {other:?}"),
    };
    assert!(!editor.is_dirty());
    assert!(!editor.diff_preview(8).is_empty());

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
