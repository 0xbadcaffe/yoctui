use super::*;

#[test]
fn raw_pty_effect_maps_to_confirmed_request_and_bounded_dimensions() {
    let request = yoctui_model::RawConfirmedExecutionRequest {
        id: yoctui_model::RawRequestId::new("raw-request:client-runtime-pty").unwrap(),
        catalog_version: 1,
        command: yoctui_model::RawCommandId::new("ui.knotty").unwrap(),
        parameters: std::collections::BTreeMap::new(),
        additional_arguments: Vec::new(),
        interaction: yoctui_model::RawInteractionMode::InteractivePty,
        safety: yoctui_model::RawSafetyClass::Build,
        capability_generation: 4,
        build_directory: "/work/build".into(),
        preview_digest: yoctui_model::RawPreviewDigest([4; 32]),
    };
    let app = App::new(16, 4096);
    let Some(DaemonCommand::StartRawPty {
        request: wire,
        dimensions,
    }) = daemon_command_for_effect(&app, &Effect::StartRaw(request.clone())).unwrap()
    else {
        panic!("expected typed Raw PTY start command");
    };
    assert_eq!(
        yoctui_app::raw_execution_request_from_protocol(&wire).unwrap(),
        request
    );
    assert!(dimensions.columns <= 512 && dimensions.rows <= 512);
}
