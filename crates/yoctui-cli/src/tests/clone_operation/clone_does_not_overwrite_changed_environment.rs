use super::*;

#[tokio::test]
async fn clone_does_not_overwrite_changed_environment() {
    let mut app = App::new(100, 4096);
    let task = tokio::spawn(async { Ok(()) });
    let mut slot = Some(fixture(&app, task));
    app.build_environment = BuildEnvironmentState::Unconfigured;
    while !slot.as_ref().unwrap().task.is_finished() {
        tokio::task::yield_now().await;
    }
    poll(&mut app, &mut slot).await;
    assert_eq!(app.build_environment, BuildEnvironmentState::Unconfigured);
    assert!(app.notification.unwrap().contains("Environment changed"));
}
