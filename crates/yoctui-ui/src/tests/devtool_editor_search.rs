use super::*;

#[test]
fn devtool_editor_search_renders_scope_specific_controls_and_workspace_results() {
    let mut app = App::new(32, 8_192);
    let _ = update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "busybox".into(),
            root: "/workspace/busybox".into(),
            files: vec!["src/main.c".into()],
        },
    );
    let editor = rendered_text(&app, 160, 50);
    for anchor in ["Ctrl+F file", "Ctrl+Shift+F workspace", "/ global"] {
        assert!(editor.contains(anchor), "missing {anchor:?}:\n{editor}");
    }

    let _ = update(&mut app, Action::OpenRecipeEditorWorkspaceSearch);
    for character in "needle".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }
    let _ = update(&mut app, Action::BeginGlobalContentSearch);
    let search = rendered_text(&app, 160, 50);
    assert!(search.contains("Workspace Regex Search"), "{search}");
    assert!(
        search.contains("Searching workspace text files"),
        "{search}"
    );
}
