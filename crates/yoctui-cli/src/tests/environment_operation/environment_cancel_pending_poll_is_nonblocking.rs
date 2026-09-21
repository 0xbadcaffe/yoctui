use super::*;

#[tokio::test]
async fn environment_cancel_pending_poll_is_nonblocking() {
    let mut app = App::new(10, 1024);
    let mut backend: Box<dyn BitBakeBackend> = Box::new(ProcessBackend::new("/".into()));
    let mut slot = Some(EnvironmentOperation {
        task: tokio::spawn(std::future::pending()),
        generation: 1,
    });
    tokio::time::timeout(
        Duration::from_millis(50),
        poll(&mut app, &mut backend, &mut slot),
    )
    .await
    .unwrap();
    assert!(slot.is_some());
    let handle = slot.as_ref().unwrap().task.abort_handle();
    drop(slot);
    tokio::task::yield_now().await;
    assert!(handle.is_finished());
}
