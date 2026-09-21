include!("compatibility_fixtures/types_and_resolution.rs");

include!("compatibility_fixtures/release_catalog.rs");

include!("compatibility_fixtures/fixture_helpers.rs");

#[cfg(test)]
mod tests {
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
}
