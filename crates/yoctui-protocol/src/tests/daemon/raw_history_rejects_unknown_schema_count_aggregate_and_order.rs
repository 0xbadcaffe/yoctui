use super::*;

#[test]
fn raw_history_rejects_unknown_schema_count_aggregate_and_order() {
    let record = RawHistoryRecordData::from_terminal(&terminal_raw_snapshot(
        1,
        RawExecutionOutcomeData::Succeeded,
    ))
    .unwrap();
    let mut future = record.clone();
    future.schema_version += 1;
    assert!(matches!(
        future.validate(),
        Err(RawExecutionProtocolError::UnsupportedHistorySchema(_))
    ));

    let mut too_many = Vec::new();
    for index in 0..=MAX_RAW_HISTORY_RECORDS {
        let mut item = record.clone();
        item.request_id = format!("raw-request:bounded-{index}");
        too_many.push(item);
    }
    assert_eq!(
        validate_raw_history_records(&too_many),
        Err(RawExecutionProtocolError::TooManyHistoryRecords)
    );

    let mut oversized = Vec::new();
    for index in 0..MAX_RAW_HISTORY_RECORDS {
        let mut item = record.clone();
        item.request_id = format!("raw-request:large-{index}");
        item.parameters = vec![RawExecutionParameterData {
            id: "text".into(),
            value: RawParameterValueData::Text("x".repeat(MAX_RAW_EXECUTION_PARAMETER_BYTES)),
        }];
        oversized.push(item);
    }
    assert_eq!(
        validate_raw_history_records(&oversized),
        Err(RawExecutionProtocolError::HistoryTooLarge)
    );

    let mut misordered = vec![record.clone(), record];
    misordered[0].request_id = "raw-request:older".into();
    misordered[0].ended_unix_ms -= 1;
    misordered[1].request_id = "raw-request:newer".into();
    assert_eq!(
        validate_raw_history_records(&misordered),
        Err(RawExecutionProtocolError::InvalidHistoryOrder)
    );
}
