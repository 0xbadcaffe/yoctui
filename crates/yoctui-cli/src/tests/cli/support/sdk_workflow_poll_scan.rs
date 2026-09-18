use super::*;

pub(crate) async fn sdk_workflow_poll_scan(
    app: &mut App,
    operation: &mut Option<SdkArtifactBackgroundOperation>,
) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while operation.is_some() {
            poll_sdk_artifact_operation(app, operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
