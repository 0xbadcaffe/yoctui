use super::*;

#[test]
fn raw_history_install_is_bounded_newest_first_and_replaces_duplicate_requests() {
    let mut older = RawHistoryRecord::from_terminal(&terminal_for_history(
        "duplicate",
        RawExecutionOutcome::Failed,
        120,
    ))
    .unwrap();
    let replacement = RawHistoryRecord::from_terminal(&terminal_for_history(
        "duplicate",
        RawExecutionOutcome::Succeeded,
        180,
    ))
    .unwrap();
    older.ended_unix_ms = 120;
    let newest = RawHistoryRecord::from_terminal(&terminal_for_history(
        "newest",
        RawExecutionOutcome::Lost,
        200,
    ))
    .unwrap();
    let mut history = Vec::new();
    install_raw_history(&mut history, [older, newest.clone(), replacement.clone()]).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0], newest);
    assert_eq!(history[1], replacement);

    let many = (0..MAX_RAW_HISTORY_RECORDS + 20).map(|index| {
        let mut record = replacement.clone();
        record.request_id = RawRequestId::new(format!("raw-request:item-{index}")).unwrap();
        record.ended_unix_ms = 100 + index as u64;
        record
    });
    install_raw_history(&mut history, many).unwrap();
    assert_eq!(history.len(), MAX_RAW_HISTORY_RECORDS);
    assert!(
        history
            .windows(2)
            .all(|pair| pair[0].ended_unix_ms >= pair[1].ended_unix_ms)
    );
}
