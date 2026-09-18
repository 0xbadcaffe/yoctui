use super::*;

#[cfg(unix)]
#[tokio::test]
async fn wic_workspace_cli_reports_rejection_and_unexpected_runner_loss() {
    let (reject_directory, _, mut rejected) = wic_workspace_fixture("reject", "sleep 30").await;
    let (reject_id, _) = wic_workspace_start_effect(&mut rejected);
    let _ = update(
        &mut rejected,
        Action::WicSessionStarting {
            id: reject_id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut rejected, Action::WicSessionRunning { id: reject_id });
    let _ = update(&mut rejected, Action::BeginActiveWicSessionCancellation);
    let Some(Effect::CancelWicSession(effect_id)) = update(
        &mut rejected,
        Action::ConfirmWicSessionCancellation {
            id: reject_id,
            acknowledge_incomplete_device: false,
        },
    ) else {
        panic!("Wic rejection effect");
    };
    let mut no_operation = None;
    begin_wic_cancellation(&mut rejected, &mut no_operation, effect_id);
    let job = rejected
        .background_jobs
        .get(rejected.wic_session(reject_id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Running);
    fs::remove_dir_all(reject_directory).unwrap();

    let (lost_directory, lost_build, mut lost) = wic_workspace_fixture("lost", "sleep 30").await;
    let (lost_id, lost_request) = wic_workspace_start_effect(&mut lost);
    let mut lost_operation = None;
    begin_wic_job(
        &mut lost,
        &mut lost_operation,
        &WicDeviceInspector::default(),
        &lost_build,
        Duration::from_millis(100),
        lost_id,
        lost_request,
    )
    .await;
    poll_wic_job(&mut lost, &mut lost_operation).await;
    poll_wic_job(&mut lost, &mut lost_operation).await;
    let lost_handle = tokio::spawn(async {
        std::future::pending::<(WicJobRunner, Result<bool, WicAdapterError>)>().await
    });
    lost_handle.abort();
    lost_operation.as_mut().unwrap().cancellation = Some(lost_handle);
    tokio::task::yield_now().await;
    poll_wic_job(&mut lost, &mut lost_operation).await;
    let job = lost
        .background_jobs
        .get(lost.wic_session(lost_id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Lost);
    fs::remove_dir_all(lost_directory).unwrap();
}
