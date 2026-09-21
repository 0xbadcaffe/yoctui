use super::*;

#[test]
fn raw_execution_protocol_round_trips_request_event_chunk_snapshot_and_result() {
    let request = raw_execution_request_fixture();
    request.validate().unwrap();
    let request_round_trip: RawExecutionRequestData =
        serde_json::from_slice(&serde_json::to_vec(&request).unwrap()).unwrap();
    assert_eq!(request_round_trip, request);

    let chunk = raw_execution_chunk_fixture();
    chunk.validate().unwrap();
    let chunk_round_trip: RawOutputChunkData =
        serde_json::from_slice(&serde_json::to_vec(&chunk).unwrap()).unwrap();
    assert_eq!(chunk_round_trip, chunk);

    let result = RawExecutionResultData {
        schema_version: RAW_EXECUTION_SCHEMA_VERSION,
        outcome: RawExecutionOutcomeData::Cancelled,
        exit_code: None,
        message: Some("cancelled by client".into()),
        elapsed_ms: 40,
        durable_reference: Some("raw-durable:history-1".into()),
    };
    result.validate().unwrap();
    let result_round_trip: RawExecutionResultData =
        serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    assert_eq!(result_round_trip, result);

    let event = RawExecutionEventData {
        schema_version: RAW_EXECUTION_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        sequence: 2,
        generation: 9,
        event: RawExecutionEventKindData::Finished { result },
    };
    event.validate().unwrap();
    let event_round_trip: RawExecutionEventData =
        serde_json::from_slice(&serde_json::to_vec(&event).unwrap()).unwrap();
    assert_eq!(event_round_trip, event);

    let snapshot = raw_execution_snapshot_fixture(4);
    snapshot.validate().unwrap();
    let snapshot_round_trip: RawExecutionSnapshotData =
        serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
    assert_eq!(snapshot_round_trip, snapshot);

    let command = ClientMessage::Command(CommandRequest {
        request_id: RequestId(4),
        expected_generation: Some(8),
        command: DaemonCommand::StartRaw { request },
    });
    assert_eq!(
        decode_frame::<ClientMessage>(&encode_frame(&command).unwrap()).unwrap(),
        command
    );
}
