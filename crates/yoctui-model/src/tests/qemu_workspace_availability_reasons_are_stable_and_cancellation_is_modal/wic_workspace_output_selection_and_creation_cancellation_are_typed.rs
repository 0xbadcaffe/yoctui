use super::*;

#[test]
fn wic_workspace_output_selection_and_creation_cancellation_are_typed() {
    let mut app = qemu_model_app();
    app.wic_capability = wic_model_capability();
    let _ = update(&mut app, Action::BeginSelectedWicCreate);
    let _ = update(&mut app, Action::PreviewWicCreate);
    let Some(Effect::StartWicSession { id, .. }) = update(&mut app, Action::ConfirmWicCreate)
    else {
        panic!("Wic start");
    };
    let _ = update(
        &mut app,
        Action::WicSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::WicSessionRunning { id });
    let _ = update(&mut app, Action::BeginActiveImageRuntimeCancellation);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicCancellationConfirmation {
            id: candidate,
            incomplete_device_warning: false,
        }) if *candidate == id
    ));
    let _ = update(&mut app, Action::CancelWicSessionCancellation);
    assert!(app.active_dialog().is_none());

    let output = WicOutput {
        identity: WicOutputIdentity {
            path: "/build/out/image.wic".into(),
            size_bytes: 1,
            modified_unix_seconds: 1,
        },
        kind: WicOutputKind::Wic,
    };
    app.wic_outputs = WicOutputInventoryState::Available {
        request: WicOutputInventoryRequest {
            generation: 1,
            output_directory: "/build/out".into(),
        },
        outputs: vec![output.clone()],
    };
    let _ = update(&mut app, Action::SelectWicOutput { delta: 1 });
    assert_eq!(app.wic_output_selection, Some(output.identity.clone()));
    assert_eq!(
        update(&mut app, Action::OpenSelectedWicOutput),
        Some(Effect::OpenInEditor(output.identity.path))
    );
}
