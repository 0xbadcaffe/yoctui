use super::*;

#[test]
fn compatibility_api_rejects_stale_or_unoffered_bridge_negotiation() {
    let snapshot = authority(
        11,
        "2.18",
        &[(
            CapabilityId::BitBakeWorkspaceInspection,
            "tinfoil.workspace",
        )],
    );
    let mut api = BitBakeApiAuthority::new(snapshot, 11, Path::new("/work/build")).unwrap();
    assert!(matches!(
        api.accept_negotiation(Some(10), &[]),
        Err(BitBakeApiCompatibilityError::NegotiationGeneration { .. })
    ));
    assert!(matches!(
        api.accept_negotiation(Some(11), &["bitbake.build".into()]),
        Err(BitBakeApiCompatibilityError::UnexpectedNegotiated(_))
    ));
}
