use super::*;

#[test]
fn raw_job_start_and_cancel_preserve_request_correlation() {
    let request = raw_execution_request_fixture();
    let request_id = request.request_id.clone();
    let start = ClientMessage::Command(CommandRequest {
        request_id: RequestId(41),
        expected_generation: Some(9),
        command: DaemonCommand::StartRaw { request },
    });
    let decoded = decode_frame::<ClientMessage>(&encode_frame(&start).unwrap()).unwrap();
    assert_eq!(decoded, start);

    let cancel = ClientMessage::Command(CommandRequest {
        request_id: RequestId(42),
        expected_generation: Some(10),
        command: DaemonCommand::CancelRaw { request_id },
    });
    assert_eq!(
        decode_frame::<ClientMessage>(&encode_frame(&cancel).unwrap()).unwrap(),
        cancel
    );
}
