use super::*;

#[cfg(unix)]
#[tokio::test]
async fn wic_workspace_cli_reports_failure_graceful_and_forced_cancellation() {
    let (failed_directory, failed_build, mut failed) =
        wic_workspace_fixture("failure", "printf 'failed\\n' >&2; exit 9").await;
    let (failed_id, failed_request) = wic_workspace_start_effect(&mut failed);
    let mut failed_operation = None;
    begin_wic_job(
        &mut failed,
        &mut failed_operation,
        &WicDeviceInspector::default(),
        &failed_build,
        Duration::from_millis(100),
        failed_id,
        failed_request,
    )
    .await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while failed_operation.is_some() {
            poll_wic_job(&mut failed, &mut failed_operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let job = failed
        .background_jobs
        .get(failed.wic_session(failed_id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Failed);
    assert_eq!(failed.wic_session(failed_id).unwrap().exit_code, Some(9));
    fs::remove_dir_all(failed_directory).unwrap();

    for (name, body, expect_forced) in [
        (
            "graceful-cancel",
            "trap 'exit 0' TERM; printf 'ready\\n'; while :; do :; done",
            false,
        ),
        (
            "forced-cancel",
            "trap '' TERM; printf 'ready\\n'; while :; do sleep 1; done",
            true,
        ),
    ] {
        let (directory, build_dir, mut app) = wic_workspace_fixture(name, body).await;
        let (id, request) = wic_workspace_start_effect(&mut app);
        let mut operation = None;
        begin_wic_job(
            &mut app,
            &mut operation,
            &WicDeviceInspector::default(),
            &build_dir,
            Duration::from_millis(50),
            id,
            request,
        )
        .await;
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                poll_wic_job(&mut app, &mut operation).await;
                let ready = app
                    .wic_session(id)
                    .and_then(|session| app.background_jobs.get(session.background_job_id))
                    .is_some_and(|job| job.output.iter().any(|entry| entry.message == "ready"));
                if ready {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let _ = update(&mut app, Action::BeginActiveWicSessionCancellation);
        let Some(Dialog::WicCancellationConfirmation {
            id: dialog_id,
            incomplete_device_warning,
        }) = app.active_dialog().cloned()
        else {
            panic!("Wic cancellation dialog");
        };
        let effect = wic_cancellation_confirmation_action(
            dialog_id,
            incomplete_device_warning,
            Input::Enter,
        )
        .and_then(|action| update(&mut app, action));
        let Some(Effect::CancelWicSession(effect_id)) = effect else {
            panic!("Wic cancellation effect");
        };
        begin_wic_cancellation(&mut app, &mut operation, effect_id);
        tokio::time::timeout(Duration::from_secs(2), async {
            while operation.is_some() {
                poll_wic_job(&mut app, &mut operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let job = app
            .background_jobs
            .get(app.wic_session(id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Cancelled);
        assert_eq!(
            job.output
                .iter()
                .any(|entry| entry.message.contains("forced termination")),
            expect_forced
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
