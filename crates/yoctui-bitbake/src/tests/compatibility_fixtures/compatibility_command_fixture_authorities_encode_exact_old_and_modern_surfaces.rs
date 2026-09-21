use super::*;

#[test]
fn compatibility_command_fixture_authorities_encode_exact_old_and_modern_surfaces() {
    let fixtures = release_capability_fixtures();
    let old = fixtures
        .iter()
        .find(|fixture| fixture.role == CompatibilityFixtureRole::OldestPolicyCandidate)
        .unwrap()
        .command_authority(41);
    let modern = fixtures
        .iter()
        .find(|fixture| fixture.role == CompatibilityFixtureRole::LatestSupportCandidate)
        .unwrap()
        .command_authority(42);

    assert_eq!(
        old.implementations[&CapabilityId::BitBakeGetVar].id,
        "bitbake.environment_lookup"
    );
    assert!(!old.snapshot.allows(CapabilityId::DevtoolUpgrade));
    assert!(!old.snapshot.allows(CapabilityId::RecipetoolCreateOutfile));
    assert!(
        !old.snapshot
            .allows(CapabilityId::BitBakeLayersCreateAndAddLayer)
    );
    assert_eq!(
        modern.implementations[&CapabilityId::BitBakeGetVar].id,
        "bitbake_getvar.argv"
    );
    for id in [
        CapabilityId::DevtoolUpgrade,
        CapabilityId::RecipetoolCreateOutfile,
        CapabilityId::BitBakeLayersCreateAndAddLayer,
    ] {
        assert!(modern.snapshot.allows(id), "{}", id.as_str());
        assert!(modern.implementations.contains_key(&id), "{}", id.as_str());
    }
}
