use super::*;

#[tokio::test]
async fn decompiled_dts_opens_document_at_start_and_scopes_search_to_current_file() {
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().join("board.dts");
    fs::write(&path, "/dts-v1/;\n/ { model = \"board\"; };\n").unwrap();
    let mut app = App::new(32, 8192);
    open_single_workspace_file(&mut app, "Images decompiled DTS".into(), path.clone()).await;
    let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
        panic!("editor missing");
    };
    assert_eq!(
        editor.context,
        yoctui_model::SourceEditorContext::DeviceTree
    );
    assert_eq!(editor.focus, yoctui_model::RecipeEditorFocus::Document);
    assert_eq!(editor.document.position().line, 0);
    assert_eq!(editor.document.position().column, 0);
    assert_eq!(editor.language, yoctui_model::SourceLanguage::DeviceTree);
    assert!(!editor.is_dirty());
    update(&mut app, Action::OpenRecipeEditorWorkspaceSearch);
    let plan = GlobalSearchPlan::for_app(&app, fixture.path(), "board".into());
    assert_eq!(plan.file, Some(path));
}

#[test]
fn workspace_editor_discovers_more_than_the_old_visible_file_limit() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-workspace-editor-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("src")).unwrap();
    for index in 0..600 {
        fs::write(
            root.join("src").join(format!("file-{index:03}.c")),
            "int value;\n",
        )
        .unwrap();
    }
    let files = recipe_editor_files(&root).unwrap();
    assert_eq!(files.len(), 600);
    assert!(files.contains(&PathBuf::from("src/file-599.c")));
    fs::remove_dir_all(root).unwrap();
}
