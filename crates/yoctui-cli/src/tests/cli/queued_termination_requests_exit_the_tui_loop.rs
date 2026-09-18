use super::*;

#[cfg(unix)]
#[tokio::test]
async fn queued_termination_requests_exit_the_tui_loop() {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    sender.send(()).await.unwrap();
    assert!(termination_requested(&mut receiver));
}
