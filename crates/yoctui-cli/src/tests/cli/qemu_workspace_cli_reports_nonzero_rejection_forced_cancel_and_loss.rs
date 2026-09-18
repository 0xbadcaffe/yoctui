use super::*;

#[cfg(unix)]
#[tokio::test]
async fn qemu_workspace_cli_reports_nonzero_rejection_forced_cancel_and_loss() {
    let (failed_directory, build_dir, mut failed) =
        qemu_workspace_fixture("failure", "printf 'failed\\n' >&2; exit 9");
    let (failed_id, failed_request) = qemu_workspace_start_effect(&mut failed);
    let mut failed_operation = None;
    begin_qemu_job(
        &mut failed,
        &mut failed_operation,
        &build_dir,
        Duration::from_millis(100),
        failed_id,
        failed_request,
    )
    .await;
    poll_qemu_until(&mut failed, &mut failed_operation, |_, operation| {
        operation.is_none()
    })
    .await;
    let failed_job = failed
        .background_jobs
        .get(failed.qemu_session(failed_id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(failed_job.status, yoctui_model::BackgroundJobStatus::Failed);
    assert_eq!(failed.qemu_session(failed_id).unwrap().exit_code, Some(9));
    fs::remove_dir_all(failed_directory).unwrap();

    let (cancel_directory, cancel_build_dir, mut cancelled) = qemu_workspace_fixture(
        "cancel",
        "trap '' TERM; printf 'ready\\n'; while :; do :; done",
    );
    let (cancel_id, cancel_request) = qemu_workspace_start_effect(&mut cancelled);
    let mut cancel_operation = None;
    begin_qemu_job(
        &mut cancelled,
        &mut cancel_operation,
        &cancel_build_dir,
        Duration::from_millis(100),
        cancel_id,
        cancel_request,
    )
    .await;
    poll_qemu_until(&mut cancelled, &mut cancel_operation, |cancelled, _| {
        cancelled
            .qemu_session(cancel_id)
            .and_then(|session| cancelled.background_jobs.get(session.background_job_id))
            .is_some_and(|job| job.output.iter().any(|entry| entry.message == "ready"))
    })
    .await;
    let _ = update(
        &mut cancelled,
        Action::BeginQemuSessionCancellation { id: cancel_id },
    );
    let Some(Effect::CancelQemuSession(effect_id)) =
        update(&mut cancelled, Action::ConfirmQemuSessionCancellation)
    else {
        panic!("cancel effect");
    };
    begin_qemu_cancellation(&mut cancelled, &mut cancel_operation, effect_id);
    poll_qemu_until(&mut cancelled, &mut cancel_operation, |_, operation| {
        operation.is_none()
    })
    .await;
    let cancel_job = cancelled
        .background_jobs
        .get(cancelled.qemu_session(cancel_id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(
        cancel_job.status,
        yoctui_model::BackgroundJobStatus::Cancelled
    );
    assert!(
        cancel_job
            .output
            .iter()
            .any(|entry| entry.message.contains("forced termination"))
    );

    let (reject_directory, _, mut rejected) = qemu_workspace_fixture("reject", "sleep 30");
    let (reject_id, _) = qemu_workspace_start_effect(&mut rejected);
    let _ = update(
        &mut rejected,
        Action::QemuSessionStarting {
            id: reject_id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut rejected, Action::QemuSessionRunning { id: reject_id });
    let _ = update(
        &mut rejected,
        Action::BeginQemuSessionCancellation { id: reject_id },
    );
    let Some(Effect::CancelQemuSession(reject_effect_id)) =
        update(&mut rejected, Action::ConfirmQemuSessionCancellation)
    else {
        panic!("reject effect");
    };
    let mut no_operation = None;
    begin_qemu_cancellation(&mut rejected, &mut no_operation, reject_effect_id);
    let reject_job = rejected
        .background_jobs
        .get(rejected.qemu_session(reject_id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(
        reject_job.status,
        yoctui_model::BackgroundJobStatus::Running
    );
    fs::remove_dir_all(reject_directory).unwrap();

    let (lost_directory, lost_build_dir, mut lost) = qemu_workspace_fixture("lost", "sleep 30");
    let (lost_id, lost_request) = qemu_workspace_start_effect(&mut lost);
    let mut lost_operation = None;
    begin_qemu_job(
        &mut lost,
        &mut lost_operation,
        &lost_build_dir,
        Duration::from_millis(100),
        lost_id,
        lost_request,
    )
    .await;
    poll_qemu_job(&mut lost, &mut lost_operation).await;
    poll_qemu_job(&mut lost, &mut lost_operation).await;
    let active = lost_operation.as_mut().unwrap();
    drop(active.runner.take());
    active.cancellation = Some(tokio::spawn(async {
        panic!("synthetic cancellation task loss");
    }));
    tokio::task::yield_now().await;
    poll_qemu_job(&mut lost, &mut lost_operation).await;
    let lost_job = lost
        .background_jobs
        .get(lost.qemu_session(lost_id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(lost_job.status, yoctui_model::BackgroundJobStatus::Lost);
    fs::remove_dir_all(lost_directory).unwrap();
    fs::remove_dir_all(cancel_directory).unwrap();
}
