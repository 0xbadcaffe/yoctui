use super::*;

#[test]
fn compatibility_api_rejects_stale_environment_command_fallback_and_missing_negotiation() {
    let snapshot = authority(
        8,
        "2.18",
        &[(CapabilityId::BitBakeGetVar, "bitbake.environment_lookup")],
    );
    assert!(matches!(
        BitBakeApiAuthority::new(snapshot.clone(), 7, Path::new("/work/build")),
        Err(BitBakeApiCompatibilityError::StaleGeneration { .. })
    ));
    assert!(matches!(
        BitBakeApiAuthority::new(snapshot.clone(), 8, Path::new("/other/build")),
        Err(BitBakeApiCompatibilityError::EnvironmentMismatch)
    ));
    let mut api = BitBakeApiAuthority::new(snapshot, 8, Path::new("/work/build")).unwrap();
    api.accept_negotiation(Some(8), &[]).unwrap();
    assert!(matches!(
        api.require(BitBakeApiOperation::Variable),
        Err(BitBakeApiCompatibilityError::ImplementationMismatch { .. })
    ));

    let direct = authority(
        9,
        "2.18",
        &[(CapabilityId::BitBakeGetVar, "tinfoil.getvar")],
    );
    let mut api = BitBakeApiAuthority::new(direct, 9, Path::new("/work/build")).unwrap();
    api.accept_negotiation(Some(9), &[]).unwrap();
    assert!(matches!(
        api.require(BitBakeApiOperation::Variable),
        Err(BitBakeApiCompatibilityError::NotNegotiated { .. })
    ));
}
