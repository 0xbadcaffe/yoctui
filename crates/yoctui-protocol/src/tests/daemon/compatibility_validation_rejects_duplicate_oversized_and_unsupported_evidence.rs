use super::*;

#[test]
fn compatibility_validation_rejects_duplicate_oversized_and_unsupported_evidence() {
    let mut duplicate = compatibility_snapshot_fixture(1);
    duplicate
        .capabilities
        .push(duplicate.capabilities[0].clone());
    assert!(matches!(
        duplicate.validate(),
        Err(CompatibilityProtocolError::DuplicateCapability(_))
    ));

    let mut oversized = compatibility_snapshot_fixture(1);
    oversized.capabilities[0].evidence[0].argv =
        vec!["argument".into(); MAX_COMPATIBILITY_ARGV + 1];
    assert_eq!(
        oversized.validate(),
        Err(CompatibilityProtocolError::Oversized("evidence argv"))
    );

    let mut contradicted = compatibility_snapshot_fixture(1);
    contradicted.capabilities[0].state = CompatibilityStateData::Unavailable {
        reason: CompatibilityReasonData {
            code: "command_missing".into(),
            message: "The command is unavailable.".into(),
            requirement: Some("bitbake-getvar --value".into()),
        },
    };
    contradicted.capabilities[0].implementation = None;
    assert!(matches!(
        contradicted.validate(),
        Err(CompatibilityProtocolError::EvidenceMismatch(_))
    ));
}
