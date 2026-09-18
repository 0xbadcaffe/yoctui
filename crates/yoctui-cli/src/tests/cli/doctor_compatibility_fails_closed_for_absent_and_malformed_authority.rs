use super::*;

#[test]
fn doctor_compatibility_fails_closed_for_absent_and_malformed_authority() {
    let absent = doctor_compatibility_report(None, Some("daemon is disconnected"));
    assert_eq!(absent.authority, DoctorCompatibilityAuthority::Unavailable);
    assert_eq!(
        absent.authority_reason.as_deref(),
        Some("daemon is disconnected")
    );
    assert!(absent.capabilities.is_empty());

    let mut malformed = doctor_compatibility_fixture();
    malformed
        .capabilities
        .push(malformed.capabilities[0].clone());
    let invalid = doctor_compatibility_report(Some(&malformed), None);
    assert_eq!(invalid.authority, DoctorCompatibilityAuthority::Invalid);
    assert!(
        invalid
            .authority_reason
            .as_deref()
            .unwrap()
            .contains("duplicate")
    );
    assert!(invalid.capabilities.is_empty());

    let mut unknown_wire = doctor_compatibility_fixture();
    unknown_wire.capabilities[0].state =
        yoctui_protocol::daemon::CompatibilityStateData::UnknownWireState;
    unknown_wire.capabilities[0].implementation = None;
    let invalid = doctor_compatibility_report(Some(&unknown_wire), None);
    assert_eq!(invalid.authority, DoctorCompatibilityAuthority::Invalid);
    assert!(
        invalid
            .authority_reason
            .as_deref()
            .unwrap()
            .contains("unknown protocol values")
    );

    let cli = Cli::try_parse_from(["yoctui", "doctor", "--json"]).unwrap();
    assert!(matches!(cli.command, Some(Command::Doctor { json: true })));
}
