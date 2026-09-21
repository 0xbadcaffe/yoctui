use super::*;

#[tokio::test]
async fn clone_pending_poll_does_not_wait_and_cancel_clears_activity() {
    let mut app = App::new(100, 4096);
    let task = tokio::spawn(async { std::future::pending::<Result<(), String>>().await });
    let mut slot = Some(fixture(&app, task));
    activity(&mut app, true);
    tokio::time::timeout(Duration::from_millis(50), poll(&mut app, &mut slot))
        .await
        .unwrap();
    assert!(slot.is_some());
    assert!(app.transient_status().unwrap().text.contains("Cloning…"));
    cancel(&mut app, &mut slot);
    assert!(slot.is_none());
    assert!(app.background_activities.is_empty());
}
