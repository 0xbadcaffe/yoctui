use super::*;

#[test]
fn round_trip() {
    let e = Envelope {
        protocol_version: 1,
        sequence: 1,
        correlation_id: Some("x".into()),
        message: Command::Hello {
            compatibility: None,
        },
    };
    assert_eq!(
        decode_line::<Command>(&encode_line(&e).unwrap(), None).unwrap(),
        e
    )
}
