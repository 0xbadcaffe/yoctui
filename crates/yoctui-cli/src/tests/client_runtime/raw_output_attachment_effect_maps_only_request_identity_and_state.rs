use super::*;

#[test]
fn raw_output_attachment_effect_maps_only_request_identity_and_state() {
    let app = App::new(16, 4096);
    let request = yoctui_model::RawRequestId::new("raw-request:client-output").unwrap();
    assert_eq!(
        daemon_command_for_effect(
            &app,
            &Effect::SetRawAttachment {
                request: request.clone(),
                attached: false,
            },
        )
        .unwrap(),
        Some(DaemonCommand::SetRawAttachment {
            request_id: request.as_str().into(),
            attached: false,
        })
    );
}
