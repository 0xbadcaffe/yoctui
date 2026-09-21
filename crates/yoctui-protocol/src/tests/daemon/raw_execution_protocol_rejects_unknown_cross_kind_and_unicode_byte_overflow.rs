use super::*;

#[test]
fn raw_execution_protocol_rejects_unknown_cross_kind_and_unicode_byte_overflow() {
    let mut request = raw_execution_request_fixture();
    request.request_id = "raw-job:protocol-1".into();
    assert_eq!(
        request.validate(),
        Err(RawExecutionProtocolError::InvalidIdentity("request"))
    );

    let mut request = raw_execution_request_fixture();
    request.interaction = serde_json::from_str("\"future_mode\"").unwrap();
    assert_eq!(
        request.validate(),
        Err(RawExecutionProtocolError::UnknownRequiredVariant)
    );

    let mut request = raw_execution_request_fixture();
    request.additional_arguments = vec!["界".repeat(MAX_RAW_EXECUTION_ARGUMENT_BYTES / 3 + 1)];
    assert_eq!(
        request.validate(),
        Err(RawExecutionProtocolError::InvalidArguments)
    );

    let future: RawExecutionEventKindData =
        serde_json::from_str(r#"{"type":"future_required"}"#).unwrap();
    assert_eq!(future, RawExecutionEventKindData::Unknown);
    let event = RawExecutionEventData {
        schema_version: RAW_EXECUTION_SCHEMA_VERSION,
        request_id: "raw-request:future".into(),
        sequence: 1,
        generation: 1,
        event: future,
    };
    assert_eq!(
        event.validate(),
        Err(RawExecutionProtocolError::UnknownRequiredVariant)
    );
}
