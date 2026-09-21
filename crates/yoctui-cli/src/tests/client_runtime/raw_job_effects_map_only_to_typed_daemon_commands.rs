use super::*;

#[test]
fn raw_job_effects_map_only_to_typed_daemon_commands() {
    let request = yoctui_model::RawConfirmedExecutionRequest {
        id: yoctui_model::RawRequestId::new("raw-request:client-runtime-1").unwrap(),
        catalog_version: 1,
        command: yoctui_model::RawCommandId::new("build.target").unwrap(),
        parameters: std::collections::BTreeMap::from([(
            yoctui_model::RawParameterId::new("target").unwrap(),
            yoctui_model::RawParameterValue::Target("core-image-minimal".into()),
        )]),
        additional_arguments: vec!["--dry-run".into()],
        interaction: yoctui_model::RawInteractionMode::NoninteractiveJob,
        safety: yoctui_model::RawSafetyClass::Build,
        capability_generation: 4,
        build_directory: "/work/build".into(),
        preview_digest: yoctui_model::RawPreviewDigest([3; 32]),
    };
    let app = App::new(16, 4096);
    let Some(DaemonCommand::StartRaw { request: wire }) =
        daemon_command_for_effect(&app, &Effect::StartRaw(request.clone())).unwrap()
    else {
        panic!("expected typed Raw start command");
    };
    assert_eq!(
        yoctui_app::raw_execution_request_from_protocol(&wire).unwrap(),
        request
    );
    assert_eq!(
        daemon_command_for_effect(&app, &Effect::CancelRaw(request.id.clone())).unwrap(),
        Some(DaemonCommand::CancelRaw {
            request_id: request.id.as_str().into(),
        })
    );
}
