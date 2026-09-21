use super::*;

#[test]
fn compatibility_fixture_catalog_has_five_unclaimed_exact_identities() {
    let fixtures = release_capability_fixtures();
    assert_eq!(fixtures.len(), CompatibilityFixtureRole::ALL.len());
    for (fixture, role) in fixtures.iter().zip(CompatibilityFixtureRole::ALL) {
        assert_eq!(fixture.role, role);
        assert!(fixture.fixture_only);
        assert_eq!(fixture.evidence_level, "deterministic_fixture_only");
        assert!(
            fixture
                .environment
                .build_directory
                .value()
                .unwrap()
                .is_absolute()
        );
        assert_eq!(
            fixture.environment.machine.value().map(String::as_str),
            Some("qemux86-64")
        );
        assert_eq!(fixture.environment.protocol.value().unwrap().version, "1.0");
    }
}
