use super::*;

#[test]
fn ux_internal_log_capture_is_typed_bounded_and_reports_channel_loss() {
    let (layer, mut capture) = bounded_channel(1);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        tracing::warn!(request = 7, value = %"日".repeat(100_000), "adapter warning");
        tracing::error!("overflowed event");
    });
    let (records, dropped) = capture.drain(8);
    assert_eq!(records.len(), 1);
    assert_eq!(dropped, 1);
    assert_eq!(records[0].level, InternalLogLevel::Warning);
    assert!(records[0].target.contains("internal_tracing"));
    assert!(records[0].message.contains("adapter warning"));
    assert!(records[0].message.len() <= MAX_CAPTURED_EVENT_BYTES);
    assert!(!records[0].message.contains('\u{fffd}'));
}
