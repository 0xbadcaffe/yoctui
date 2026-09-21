use super::*;

#[test]
fn raw_output_attachment_command_preserves_request_correlation() {
    let command = ClientMessage::Command(CommandRequest {
        request_id: RequestId(44),
        expected_generation: Some(12),
        command: DaemonCommand::SetRawAttachment {
            request_id: "raw-request:protocol-output".into(),
            attached: false,
        },
    });
    assert_eq!(
        decode_frame::<ClientMessage>(&encode_frame(&command).unwrap()).unwrap(),
        command
    );
}
