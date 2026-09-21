use super::*;

#[test]
fn rejects_sequence() {
    let v = br#"{"protocol_version":1,"sequence":2,"message":{"type":"hello"}}"#;
    assert!(matches!(
        decode_line::<Command>(v, Some(2)),
        Err(ProtocolError::Sequence { .. })
    ))
}
