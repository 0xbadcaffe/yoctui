use super::*;

#[test]
fn wic_device_write_requires_exact_phrase_and_cancellation_warning() {
    let mut app = App::new(20, 20_000);
    app.wic_capability = wic_model_capability();
    let image = WicOutputIdentity {
        path: "/build/wic-output/image.wic".into(),
        size_bytes: 1024,
        modified_unix_seconds: 1,
    };
    app.wic_outputs = WicOutputInventoryState::Available {
        request: WicOutputInventoryRequest {
            generation: 1,
            output_directory: "/build/wic-output".into(),
        },
        outputs: vec![WicOutput {
            identity: image.clone(),
            kind: WicOutputKind::Wic,
        }],
    };
    app.wic_output_selection = Some(image.clone());
    let Some(Effect::GetWicDevices(request)) =
        update(&mut app, Action::BeginSelectedWicDeviceWrite)
    else {
        panic!("Wic device discovery effect");
    };
    assert_eq!(request.image, image);
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicDevicePicker(dialog)) if dialog.request == request
    ));
    let ignored = app.background_jobs.ignored_transitions;
    let mut stale = request.clone();
    stale.generation += 1;
    let _ = update(
        &mut app,
        Action::WicDeviceInventoryLoaded {
            request: stale,
            devices: vec![wic_model_device("/dev/sdy", "8:239")],
            limitations: Vec::new(),
        },
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
    let first = wic_model_device("/dev/sdy", "8:239");
    let selected = wic_model_device("/dev/sdz", "8:240");
    let _ = update(
        &mut app,
        Action::WicDeviceInventoryLoaded {
            request: request.clone(),
            devices: vec![first, selected.clone()],
            limitations: vec!["system device excluded".into()],
        },
    );
    let _ = update(&mut app, Action::SelectWicDevice { delta: 1 });
    assert_eq!(app.wic_device_selection, Some(selected.identity.clone()));
    let _ = update(&mut app, Action::CancelWicDevicePicker);
    let Some(Effect::GetWicDevices(refreshed_request)) =
        update(&mut app, Action::BeginSelectedWicDeviceWrite)
    else {
        panic!("refreshed Wic device discovery effect");
    };
    assert!(refreshed_request.generation > request.generation);
    let _ = update(
        &mut app,
        Action::WicDeviceInventoryLoaded {
            request: refreshed_request,
            devices: vec![selected.clone(), wic_model_device("/dev/sdy", "8:239")],
            limitations: Vec::new(),
        },
    );
    assert_eq!(app.wic_device_selection, Some(selected.identity.clone()));
    let _ = update(&mut app, Action::ConfirmWicDeviceSelection);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicWritePhrase(dialog)) if dialog.device == selected.identity
    ));
    for character in "WRITE /dev/sdy".chars() {
        let _ = update(&mut app, Action::AppendWicWritePhrase(character));
    }
    assert!(update(&mut app, Action::PreviewWicDeviceWrite).is_none());
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicWritePhrase(WicWritePhraseDialog {
            validation_error: Some(_),
            ..
        }))
    ));
    if let Some(Dialog::WicWritePhrase(dialog)) = app.active_dialog_mut() {
        dialog.input = "WRITE /dev/sdz".into();
    }
    let _ = update(&mut app, Action::PreviewWicDeviceWrite);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicWriteConfirmation(_))
    ));
    app.wic_capability = WicCapability::MissingTool;
    assert!(update(&mut app, Action::ConfirmWicDeviceWrite).is_none());
    assert!(app.active_wic_session().is_none());
    app.wic_capability = WicCapability::MissingKickstarts {
        executable: "/opt/poky/scripts/wic".into(),
    };
    let Some(Effect::StartWicSession { id, operation }) =
        update(&mut app, Action::ConfirmWicDeviceWrite)
    else {
        panic!("Wic write start effect");
    };
    assert!(matches!(operation, WicOperation::Write(_)));
    assert!(app.active_dialog().is_none());
    let _ = update(
        &mut app,
        Action::WicSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::WicSessionRunning { id });
    let _ = update(&mut app, Action::BeginActiveWicSessionCancellation);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicCancellationConfirmation {
            id: candidate,
            incomplete_device_warning: true,
        }) if *candidate == id
    ));
    assert!(
        update(
            &mut app,
            Action::ConfirmWicSessionCancellation {
                id,
                acknowledge_incomplete_device: false,
            },
        )
        .is_none()
    );
    assert_eq!(
        app.background_jobs
            .get(app.wic_session(id).unwrap().background_job_id)
            .map(|job| job.status),
        Some(BackgroundJobStatus::Running)
    );
    assert_eq!(
        update(
            &mut app,
            Action::ConfirmWicSessionCancellation {
                id,
                acknowledge_incomplete_device: true,
            },
        ),
        Some(Effect::CancelWicSession(id))
    );
    let _ = update(
        &mut app,
        Action::CancelWicSession {
            id,
            exit_code: Some(130),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let job = app
        .background_jobs
        .get(app.wic_session(id).unwrap().background_job_id)
        .unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Cancelled);
    assert!(
        job.error
            .as_ref()
            .and_then(|error| error.detail.as_deref())
            .is_some_and(|detail| detail.contains("incomplete"))
    );
}
