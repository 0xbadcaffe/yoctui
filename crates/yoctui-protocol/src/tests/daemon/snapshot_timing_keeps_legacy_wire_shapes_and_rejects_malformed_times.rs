use super::*;

#[test]
fn snapshot_timing_keeps_legacy_wire_shapes_and_rejects_malformed_times() {
    let legacy = r#"{"type":"started"}"#;
    let decoded: DaemonBuildEvent = serde_json::from_str(legacy).unwrap();
    assert_eq!(
        decoded,
        DaemonBuildEvent::Started {
            started_unix_ms: None
        }
    );
    assert_eq!(serde_json::to_string(&decoded).unwrap(), legacy);
    #[derive(Debug, PartialEq, serde::Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    enum LegacyEvent {
        Started,
        Completed {
            success: bool,
            exit_code: Option<i32>,
        },
    }
    let started = DaemonBuildEvent::Started {
        started_unix_ms: Some(123),
    };
    assert_eq!(
        serde_json::from_value::<LegacyEvent>(serde_json::to_value(started).unwrap()).unwrap(),
        LegacyEvent::Started
    );
    let completed = DaemonBuildEvent::Completed {
        success: true,
        exit_code: Some(0),
        finished_unix_ms: Some(456),
    };
    assert_eq!(
        serde_json::from_value::<LegacyEvent>(serde_json::to_value(completed).unwrap()).unwrap(),
        LegacyEvent::Completed {
            success: true,
            exit_code: Some(0)
        }
    );
    for malformed in [
        r#"{"type":"started","started_unix_ms":-1}"#,
        r#"{"type":"started","started_unix_ms":1.5}"#,
        r#"{"type":"started","started_unix_ms":"now"}"#,
        r#"{"type":"started","started_unix_ms":18446744073709551616}"#,
    ] {
        assert!(
            serde_json::from_str::<DaemonBuildEvent>(malformed).is_err(),
            "{malformed}"
        );
    }
}
