use super::*;

#[test]
fn nested_build_event_round_trips_without_duplicate_type_fields() {
    let event = DaemonEvent::Build(DaemonBuildEvent::Reset {
        targets: vec!["core-image-minimal".into()],
    });
    let encoded = serde_json::to_string(&event).unwrap();
    assert_eq!(encoded.matches("\"type\"").count(), 2);
    assert_eq!(
        serde_json::from_str::<DaemonEvent>(&encoded).unwrap(),
        event
    );
}
