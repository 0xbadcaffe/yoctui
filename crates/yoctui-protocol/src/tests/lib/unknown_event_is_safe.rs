use super::*;

#[test]
fn unknown_event_is_safe() {
    let v = br#"{"protocol_version":1,"sequence":2,"message":{"type":"future_event"}}"#;
    assert_eq!(
        decode_line::<Event>(v, None).unwrap().message,
        Event::Unknown
    )
}
