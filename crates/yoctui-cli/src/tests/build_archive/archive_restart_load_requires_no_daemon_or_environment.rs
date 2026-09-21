use super::*;

#[tokio::test]
async fn archive_restart_load_requires_no_daemon_or_environment() {
    let root = Root::new();
    save(&root.0, record(1)).unwrap();
    let mut app = App::new_unconfigured(32, 4096);
    app.require_daemon = true;
    app.saved_builds.reload_requested = true;
    let mut load = None;
    assert!(poll_load(&mut app, &mut load, &root.0).await);
    tokio::time::timeout(Duration::from_secs(2), async {
        while load.is_some() {
            poll_load(&mut app, &mut load, &root.0).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(app.saved_builds.records[0].logs[0].message, "build 1");
    assert_eq!(
        app.daemon.status,
        yoctui_model::ClientReplicaStatus::Disconnected
    );
    assert!(matches!(
        app.build_environment,
        yoctui_model::BuildEnvironmentState::Unconfigured
    ));
    assert!(app.tasks.is_empty());
}
