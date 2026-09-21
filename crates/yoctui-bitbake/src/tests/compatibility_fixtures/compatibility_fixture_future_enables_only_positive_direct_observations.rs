use super::*;

#[test]
fn compatibility_fixture_future_enables_only_positive_direct_observations() {
    let fixture = release_capability_fixtures()
        .into_iter()
        .find(|fixture| fixture.role == CompatibilityFixtureRole::FutureUnknown)
        .unwrap();
    let resolved = fixture.resolve(99);
    assert!(resolved.snapshot.allows(CapabilityId::DevtoolUpgrade));
    for id in [
        CapabilityId::BitBakeBuild,
        CapabilityId::ResultTool,
        CapabilityId::WicCreate,
        CapabilityId::RunQemu,
    ] {
        assert!(!resolved.snapshot.allows(id), "{}", id.as_str());
    }
    assert!(
        resolved
            .snapshot
            .capability(CapabilityId::BitBakeBuild)
            .unwrap()
            .evidence
            .iter()
            .all(|evidence| evidence.outcome != CapabilityEvidenceOutcome::Positive)
    );
}
