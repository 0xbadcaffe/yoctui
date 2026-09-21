use super::*;

#[test]
fn raw_pty_start_preserves_only_confirmed_intent_and_dimensions() {
    let mut request = raw_execution_request_fixture();
    request.interaction = RawInteractionData::InteractivePty;
    let start = ClientMessage::Command(CommandRequest {
        request_id: RequestId(43),
        expected_generation: Some(11),
        command: DaemonCommand::StartRawPty {
            request,
            dimensions: TerminalDimensions {
                columns: 132,
                rows: 43,
            },
        },
    });
    assert_eq!(
        decode_frame::<ClientMessage>(&encode_frame(&start).unwrap()).unwrap(),
        start
    );
}
