use super::*;

#[test]
fn recipe_bitbake_action_force_flag_round_trips_and_defaults_false() {
    let envelope = Envelope {
        protocol_version: VERSION,
        sequence: 9,
        correlation_id: None,
        message: Command::StartBuild {
            targets: vec!["busybox".into()],
            task: Some("compile".into()),
            force: true,
        },
    };
    assert_eq!(
        decode_line::<Command>(&encode_line(&envelope).unwrap(), None).unwrap(),
        envelope
    );
    let old = br#"{"protocol_version":1,"sequence":10,"message":{"type":"start_build","targets":["busybox"],"task":"compile"}}"#;
    assert!(matches!(
        decode_line::<Command>(old, None).unwrap().message,
        Command::StartBuild { force: false, .. }
    ));
}
