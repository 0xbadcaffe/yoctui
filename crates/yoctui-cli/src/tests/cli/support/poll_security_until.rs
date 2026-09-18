use super::*;

pub(crate) async fn poll_security_until(
    coordinator: &mut SecurityCliCoordinator,
    app: &mut App,
    complete: impl Fn(&App, &SecurityCliCoordinator) -> bool,
) {
    for _ in 0..200 {
        coordinator.poll(app).await;
        if complete(app, coordinator) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    panic!("Security CLI operation did not finish");
}
