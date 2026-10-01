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
    for anchor in ["Ctrl+F file", "Alt+f workspace", "/ global"] {
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

#[test]
fn modifier_editor_hints_match_routes_and_render_safely() {
    let mut app = App::new(32, 8_192);
    update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "Layer: meta-test".into(),
            root: "/layers/meta-test".into(),
            files: vec!["example.bb".into()],
        },
    );
    for mode in [
        yoctui_model::TextAreaMode::Normal,
        yoctui_model::TextAreaMode::Insert,
        yoctui_model::TextAreaMode::Visual,
    ] {
        if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
            editor.focus = yoctui_model::RecipeEditorFocus::Document;
            editor.document.set_mode(mode);
        }
        let text = rendered_text(&app, 180, 56);
        assert!(text.contains("Alt+f workspace"), "{text}");
        assert!(text.contains("Alt+g GitUI"), "{text}");
        assert!(!text.contains("Ctrl+Shift+F"));
        for (width, height) in [(160, 50), (100, 30), (80, 24), (20, 8), (1, 1)] {
            let _ = rendered_text(&app, width, height);
        }
    }
}
