use super::*;

#[test]
fn compatibility_fixture_capability_differences_are_exact_and_complete() {
    for (index, fixture) in release_capability_fixtures().iter().enumerate() {
        let resolved = fixture.resolve(index as u64 + 1);
        assert_eq!(
            resolved.snapshot.capabilities.len(),
            CapabilityId::ALL.len()
        );
        for expected in &fixture.expectations {
            let record = resolved.snapshot.capability(expected.id).unwrap();
            assert_eq!(
                fixture_state(&record.state),
                expected.state,
                "{} {}",
                fixture.role.as_str(),
                expected.id.as_str()
            );
            assert_eq!(
                fixture_implementation(&resolved, expected.id).map(|value| value.id.as_str()),
                expected.implementation.as_deref(),
                "{} {}",
                fixture.role.as_str(),
                expected.id.as_str()
            );
        }
    }
}
