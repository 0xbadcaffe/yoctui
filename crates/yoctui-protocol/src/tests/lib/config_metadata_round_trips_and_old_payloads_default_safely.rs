use super::*;

#[test]
fn config_metadata_round_trips_and_old_payloads_default_safely() {
    let event = Event::Variable {
        name: "MACHINE".into(),
        recipe: Some("base-files".into()),
        value: Some("qemux86-64".into()),
        provenance: Some("/build/conf/local.conf:12".into()),
        unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
        operations: vec![VariableOperationData {
            operation: "set".into(),
            file: Some("/build/conf/local.conf".into()),
            line: Some(12),
            value: Some("${DEFAULT_MACHINE}".into()),
        }],
        active_overrides: vec!["qemux86-64".into(), "poky".into()],
    };
    let envelope = Envelope {
        protocol_version: VERSION,
        sequence: 11,
        correlation_id: Some("config-metadata".into()),
        message: event.clone(),
    };
    assert_eq!(
        decode_line::<Event>(&encode_line(&envelope).unwrap(), None)
            .unwrap()
            .message,
        event
    );

    let old = br#"{"protocol_version":1,"sequence":12,"message":{"type":"variable","name":"MACHINE","value":"qemuarm","provenance":"conf/local.conf:1"}}"#;
    assert!(matches!(
        decode_line::<Event>(old, None).unwrap().message,
        Event::Variable {
            recipe: None,
            unexpanded_value: None,
            operations,
            active_overrides,
            ..
        } if operations.is_empty() && active_overrides.is_empty()
    ));

    let malformed = br#"{"protocol_version":1,"sequence":13,"message":{"type":"variable","name":"MACHINE","value":"qemuarm","operations":[{"operation":"set","line":"twelve"}]}}"#;
    assert!(decode_line::<Event>(malformed, None).is_err());
}
