use super::*;

pub(crate) async fn sdk_workflow_poll_job(
    app: &mut App,
    operation: &mut Option<SdkCliOperation>,
) -> Option<SdkOperation> {
    tokio::time::timeout(Duration::from_secs(3), async {
        let mut completed = None;
        while operation.is_some() {
            completed = poll_sdk_job(app, operation).await.or(completed);
            tokio::task::yield_now().await;
        }
        completed
    })
    .await
    .unwrap()
}
