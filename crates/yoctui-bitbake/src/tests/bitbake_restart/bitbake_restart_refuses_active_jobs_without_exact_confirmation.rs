use super::*;

#[tokio::test]
async fn bitbake_restart_refuses_active_jobs_without_exact_confirmation() {
    let mut coordinator = coordinator().await;
    let preview = coordinator.preview(affected()).unwrap();
    assert_eq!(
        coordinator.restart(&preview, affected(), None).await,
        Err(BitBakeRestartError::ConfirmationRequired)
    );
    let mut stale = preview.confirmation();
    stale.controller_generation += 1;
    assert_eq!(
        coordinator
            .restart(&preview, affected(), Some(&stale))
            .await,
        Err(BitBakeRestartError::ConfirmationRequired)
    );
    assert_eq!(
        coordinator
            .restart(&preview, Vec::new(), Some(&preview.confirmation()))
            .await,
        Err(BitBakeRestartError::StalePreview)
    );
}
