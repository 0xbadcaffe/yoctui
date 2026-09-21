use super::*;

#[test]
fn raw_execution_app_mechanically_round_trips_request_event_chunk_result_and_snapshot() {
    let mut state = raw_execution_state_fixture();
    let request = raw_execution_request_to_protocol(&state.request).unwrap();
    assert_eq!(
        raw_execution_request_from_protocol(&request).unwrap(),
        state.request
    );
    let event = apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Starting {
            owner: yoctui_model::RawExecutionOwner::Job(
                yoctui_model::RawJobId::new("raw-job:app-1").unwrap(),
            ),
        },
    );
    let wire_event = raw_execution_event_to_protocol(&event).unwrap();
    assert_eq!(
        raw_execution_event_from_protocol(&wire_event).unwrap(),
        event
    );
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Running {
            started_unix_ms: 20,
        },
    );
    let chunk = yoctui_model::RawOutputChunk {
        stream_id: state.stdout.stream_id.clone(),
        stream: yoctui_model::RawOutputStream::Stdout,
        sequence: 1,
        text: "unicode 界\n".into(),
        truncated_bytes: 0,
        dropped_lines: 0,
    };
    let wire_chunk = raw_output_chunk_to_protocol(&chunk).unwrap();
    assert_eq!(raw_output_chunk_from_protocol(&wire_chunk).unwrap(), chunk);
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Output { chunk },
    );
    let result = yoctui_model::RawExecutionResult {
        outcome: yoctui_model::RawExecutionOutcome::Succeeded,
        exit_code: Some(0),
        message: Some("complete".into()),
        elapsed_ms: 50,
        durable_reference: Some(
            yoctui_model::RawDurableReferenceId::new("raw-durable:app-1").unwrap(),
        ),
    };
    let wire_result = raw_execution_result_to_protocol(&result).unwrap();
    assert_eq!(
        raw_execution_result_from_protocol(&wire_result).unwrap(),
        result
    );
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Finished { result },
    );
    let snapshot = raw_execution_snapshot_to_protocol(&state).unwrap();
    assert_eq!(
        raw_execution_snapshot_from_protocol(&snapshot).unwrap(),
        state
    );
}
