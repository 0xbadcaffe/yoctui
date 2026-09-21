use super::*;

#[test]
fn raw_history_records_all_terminal_outcomes_without_live_authority() {
    assert_eq!(
        RawHistoryRecordData::from_terminal(&raw_execution_snapshot_fixture(1)),
        Err(RawExecutionProtocolError::HistoryRequiresTerminal)
    );
    for outcome in [
        RawExecutionOutcomeData::Succeeded,
        RawExecutionOutcomeData::Failed,
        RawExecutionOutcomeData::Cancelled,
        RawExecutionOutcomeData::Lost,
    ] {
        let record =
            RawHistoryRecordData::from_terminal(&terminal_raw_snapshot(1, outcome)).unwrap();
        assert_eq!(record.outcome, outcome);
        let json = serde_json::to_string(&record).unwrap();
        for prohibited in [
            "raw-job:",
            "owner",
            "stdout",
            "stderr",
            "capability_generation",
            "build_directory",
            "preview_digest",
            "additional_arguments",
            "must not be retained",
        ] {
            assert!(!json.contains(prohibited), "retained {prohibited}: {json}");
        }
    }
    let mut sensitive = terminal_raw_snapshot(1, RawExecutionOutcomeData::Succeeded);
    sensitive.request.parameters.extend([
        RawExecutionParameterData {
            id: "free-text".into(),
            value: RawParameterValueData::Text("token-like-value".into()),
        },
        RawExecutionParameterData {
            id: "temporary-file".into(),
            value: RawParameterValueData::File("/tmp/private-input".into()),
        },
    ]);
    let sanitized = RawHistoryRecordData::from_terminal(&sensitive).unwrap();
    assert!(sanitized.parameters.iter().all(|parameter| !matches!(
        &parameter.value,
        RawParameterValueData::Text(_) | RawParameterValueData::File(_)
    )));
}
