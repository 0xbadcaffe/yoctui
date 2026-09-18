use super::*;

pub(crate) async fn poll_qa_until(
    coordinator: &mut QaCliCoordinator,
    app: &mut App,
    complete: impl Fn(&App, &QaCliCoordinator) -> bool,
) {
    for _ in 0..300 {
        coordinator.poll(app).await;
        if complete(app, coordinator) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    panic!("QA CLI operation did not finish");
}
