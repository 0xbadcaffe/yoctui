use super::*;

#[test]
fn daemon_protocol_negotiates_versions_and_capabilities_explicitly() {
    assert_eq!(
        negotiate_version(
            ProtocolVersion { major: 1, minor: 0 },
            ProtocolVersion { major: 1, minor: 4 },
            ProtocolVersion { major: 1, minor: 2 },
        )
        .unwrap(),
        ProtocolVersion { major: 1, minor: 2 }
    );
    assert!(matches!(
        negotiate_version(
            ProtocolVersion { major: 2, minor: 0 },
            ProtocolVersion { major: 2, minor: 0 },
            ProtocolVersion::CURRENT,
        ),
        Err(DaemonProtocolError::IncompatibleVersion)
    ));
    assert_eq!(
        negotiate_capabilities(
            &[Capability::PtySessions, Capability::StateSnapshots],
            &[Capability::StateSnapshots, Capability::BackgroundJobs],
        )
        .unwrap(),
        vec![Capability::StateSnapshots]
    );

    let future: Capability = serde_json::from_str("\"future_capability\"").unwrap();
    assert_eq!(future, Capability::Unknown);
    let future_event: DaemonEvent =
        serde_json::from_str(r#"{"type":"future_optional_event"}"#).unwrap();
    assert_eq!(future_event, DaemonEvent::Unknown);
}
