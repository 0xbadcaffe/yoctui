use super::*;

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
