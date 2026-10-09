use super::*;

#[test]
fn devtool_filter_paste_uses_metadata_query_not_terminal_or_global_search() {
    let mut app = App::new(16, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Devtool;
    app.focus = yoctui_model::FocusTarget::Workspace;
    app.metadata_searching = true;
    let actions = yoctui_app::text_paste_actions(
        &app,
        "iw".into(),
        yoctui_model::TextAreaPasteSource::BracketedPaste,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        actions,
        vec![
            Action::AppendMetadataQuery('i'),
            Action::AppendMetadataQuery('w')
        ]
    );
}

#[test]
fn workspace_search_paste_targets_overlay_without_changing_retained_editor() {
    let mut app = App::new(16, 4096);
    app.onboarding.open = false;
    update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "iw".into(),
            root: "/build/workspace/sources/iw".into(),
            files: vec!["iw.c".into()],
        },
    );
    update(
        &mut app,
        Action::LoadRecipeEditorContent("int value;".into()),
    );
    let before = app.active_dialog().cloned();
    update(&mut app, Action::OpenRecipeEditorWorkspaceSearch);
    for source in [
        yoctui_model::TextAreaPasteSource::Clipboard,
        yoctui_model::TextAreaPasteSource::BracketedPaste,
    ] {
        let actions = yoctui_app::text_paste_actions(&app, "value".into(), source)
            .unwrap()
            .unwrap();
        assert!(
            actions
                .iter()
                .all(|action| matches!(action, Action::AppendCommandPaletteQuery(_)))
        );
        for action in actions {
            update(&mut app, action);
        }
        assert_eq!(app.active_dialog(), before.as_ref());
        assert!(!app.command_palette_query.is_empty());
        update(&mut app, Action::ClearCommandPaletteQuery);
    }
}

#[tokio::test]
async fn pasted_global_search_schedules_once_and_command_palette_does_not_scan() {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    let mut operation = None;
    let fixture = tempfile::tempdir().unwrap();
    let build_dir = fixture.path();
    fs::create_dir(build_dir.join("conf")).unwrap();
    fs::write(build_dir.join("conf/local.conf"), "MACHINE = \"romulus\"\n").unwrap();
    app.workspace.build_dir = Some(build_dir.to_path_buf());
    for source in [
        yoctui_model::TextAreaPasteSource::Clipboard,
        yoctui_model::TextAreaPasteSource::BracketedPaste,
    ] {
        update(&mut app, Action::OpenGlobalSearch);
        let actions = yoctui_app::text_paste_actions(&app, "romulus".into(), source)
            .unwrap()
            .unwrap();
        apply_paste_actions(&mut app, actions, build_dir, &mut operation);
        let worker = operation
            .as_ref()
            .expect("paste must schedule content search");
        assert_eq!(worker.query, "romulus");
        assert!(
            matches!(&app.global_search_content, yoctui_model::GlobalSearchContentState::Loading { query, .. } if query == "romulus")
        );
        tokio::time::timeout(Duration::from_secs(5), async {
            while !operation.as_ref().unwrap().handle.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        poll_global_content_search(&mut app, &mut operation).await;
        assert!(
            matches!(&app.global_search_content, yoctui_model::GlobalSearchContentState::Ready { hits, .. } if !hits.is_empty())
        );
        update(&mut app, Action::CloseCommandPalette);
    }
    update(&mut app, Action::OpenCommandPalette);
    let actions = yoctui_app::text_paste_actions(
        &app,
        "romulus".into(),
        yoctui_model::TextAreaPasteSource::Clipboard,
    )
    .unwrap()
    .unwrap();
    apply_paste_actions(&mut app, actions, build_dir, &mut operation);
    assert!(operation.is_none());
}
