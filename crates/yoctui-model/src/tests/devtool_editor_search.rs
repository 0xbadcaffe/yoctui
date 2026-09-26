use super::*;

fn open_editor(app: &mut App) {
    let effect = update(
        app,
        Action::OpenRecipeEditor {
            recipe: "busybox".into(),
            root: "/workspace/busybox".into(),
            files: vec!["src/main.c".into(), "src/other.c".into()],
        },
    );
    assert_eq!(
        effect,
        Some(Effect::LoadRecipeEditorFile(
            "/workspace/busybox/src/main.c".into()
        ))
    );
    let _ = update(
        app,
        Action::LoadRecipeEditorContent("first\nneedle\n".into()),
    );
}

#[test]
fn devtool_editor_search_scopes_workspace_and_loads_selected_hit_into_editor() {
    let mut app = App::new(16, 4_096);
    open_editor(&mut app);

    let _ = update(&mut app, Action::OpenRecipeEditorWorkspaceSearch);
    assert!(app.command_palette_open);
    assert_eq!(app.global_search_root, Some("/workspace/busybox".into()));
    for character in "needle".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }
    let _ = update(&mut app, Action::BeginGlobalContentSearch);
    let generation = app.global_search_generation;
    let query = app.command_palette_query.clone();
    let hit = GlobalSearchHit {
        kind: GlobalSearchContentKind::GeneratedMetadata,
        path: "/workspace/busybox/src/other.c".into(),
        line: 7,
        column: 4,
        preview: "int needle = 1;".into(),
        image: None,
    };
    let _ = update(
        &mut app,
        Action::GlobalContentSearchLoaded {
            generation,
            query,
            hits: vec![hit],
            truncated: false,
            searched_scopes: vec!["workspace=/workspace/busybox".into()],
        },
    );
    app.command_palette_selection = app.filtered_command_palette_commands().len();
    assert_eq!(
        update(&mut app, Action::ActivateCommandPalette),
        Some(Effect::LoadRecipeEditorFile(
            "/workspace/busybox/src/other.c".into()
        ))
    );
    let _ = update(
        &mut app,
        Action::LoadRecipeEditorContent("0\n1\n2\n3\n4\n5\nneedle\n".into()),
    );
    let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
        panic!("recipe editor missing")
    };
    assert_eq!(editor.selection, 1);
    assert_eq!(
        editor.document.position(),
        TextAreaPosition { line: 6, column: 3 }
    );
    assert_eq!(editor.focus, RecipeEditorFocus::Document);
}

#[test]
fn devtool_editor_search_keeps_global_scope_and_refuses_dirty_file_switch() {
    let mut app = App::new(16, 4_096);
    open_editor(&mut app);
    let _ = update(
        &mut app,
        Action::EditRecipeEditor(PopupEditorCommand::ToggleInsert),
    );
    let _ = update(
        &mut app,
        Action::EditRecipeEditor(PopupEditorCommand::Insert('x')),
    );
    let _ = update(&mut app, Action::OpenRecipeEditorWorkspaceSearch);
    app.global_search_content = GlobalSearchContentState::Ready {
        generation: 1,
        query: "needle".into(),
        hits: vec![GlobalSearchHit {
            kind: GlobalSearchContentKind::GeneratedMetadata,
            path: "/workspace/busybox/src/other.c".into(),
            line: 1,
            column: 1,
            preview: "needle".into(),
            image: None,
        }],
        truncated: false,
        searched_scopes: vec![],
    };
    app.command_palette_selection = app.filtered_command_palette_commands().len();
    assert_eq!(update(&mut app, Action::ActivateCommandPalette), None);
    assert!(app.command_palette_open);
    assert!(app.notification.as_deref().unwrap().contains("Ctrl+S"));

    let _ = update(&mut app, Action::OpenGlobalSearch);
    assert_eq!(app.global_search_root, None);
}
