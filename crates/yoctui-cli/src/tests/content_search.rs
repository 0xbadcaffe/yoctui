use super::*;
use yoctui_model::{GlobalSearchContentKind, GlobalSearchContentState, GlobalSearchHit};

fn hit(name: &str) -> GlobalSearchHit {
    GlobalSearchHit {
        kind: GlobalSearchContentKind::GeneratedMetadata,
        path: format!("/build/{name}.txt").into(),
        line: 1,
        column: 1,
        preview: "needle".into(),
        image: None,
    }
}

#[tokio::test]
async fn streaming_search_poll_publishes_pending_worker_results_and_keeps_final_order() {
    let mut app = App::new(32, 8192);
    update(&mut app, Action::OpenGlobalSearch);
    app.command_palette_query = "needle".into();
    update(&mut app, Action::BeginGlobalContentSearch);
    let generation = app.global_search_generation;
    let (sender, progress) = tokio::sync::mpsc::unbounded_channel();
    let (finish, finished) = tokio::sync::oneshot::channel();
    let mut operation = Some(GlobalContentSearchOperation {
        generation,
        query: "needle".into(),
        cancellation: GlobalSearchCancellation::default(),
        progress,
        handle: tokio::spawn(async { finished.await.unwrap() }),
    });
    sender.send(hit("z")).unwrap();
    poll_global_content_search(&mut app, &mut operation).await;
    assert!(
        operation.is_some(),
        "must not wait for completion before displaying a match"
    );
    assert!(matches!(
        app.global_search_content,
        GlobalSearchContentState::Streaming { .. }
    ));
    assert_eq!(app.global_search_content.hits(), &[hit("z")]);
    sender.send(hit("a")).unwrap();
    poll_global_content_search(&mut app, &mut operation).await;
    assert_eq!(app.global_search_content.hits(), &[hit("z"), hit("a")]);
    app.command_palette_selection = 1;
    finish
        .send(Ok(GlobalSearchScanResult {
            hits: vec![hit("z"), hit("a")],
            truncated: false,
            searched_scopes: vec![],
        }))
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !operation.as_ref().unwrap().handle.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    poll_global_content_search(&mut app, &mut operation).await;
    assert!(operation.is_none());
    assert!(!app.global_search_content.loading());
    assert_eq!(app.global_search_content.hits(), &[hit("z"), hit("a")]);
    assert_eq!(app.command_palette_selection, 1);
}
