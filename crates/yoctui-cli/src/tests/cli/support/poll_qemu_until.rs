use super::*;

pub(crate) async fn poll_qemu_until(
    app: &mut App,
    operation: &mut Option<QemuCliOperation>,
    condition: impl Fn(&App, &Option<QemuCliOperation>) -> bool,
) {
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        while !condition(app, operation) {
            poll_qemu_job(app, operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(
        result.is_ok(),
        "timed out waiting for QEMU CLI state; operation active: {}",
        operation.is_some()
    );
}
