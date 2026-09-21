use super::*;

#[test]
fn compatibility_command_getvar_prefers_direct_utility_and_uses_environment_capability_fallback() {
    let catalog = CapabilityCatalog::builtin();
    let positive_getvar = BTreeMap::from([(
        CapabilityId::BitBakeGetVar,
        complete_observations(
            &catalog,
            CapabilityId::BitBakeGetVar,
            CapabilityProbeStatus::Positive,
            "bitbake-getvar help and required options",
        ),
    )]);
    let direct = CapabilityResolver::default()
        .resolve_snapshot(3, future_environment(), &catalog, &positive_getvar)
        .unwrap();
    assert_eq!(
        direct
            .implementations
            .get(&CapabilityId::BitBakeGetVar)
            .unwrap()
            .id,
        "bitbake_getvar.argv"
    );
    assert_eq!(
        direct
            .snapshot
            .capability(CapabilityId::BitBakeGetVar)
            .unwrap()
            .state,
        CapabilityState::Available
    );

    let environment_only = BTreeMap::from([(
        CapabilityId::BitBakeEnvironmentDump,
        complete_observations(
            &catalog,
            CapabilityId::BitBakeEnvironmentDump,
            CapabilityProbeStatus::Positive,
            "bitbake -e",
        ),
    )]);
    let fallback = CapabilityResolver::default()
        .resolve_snapshot(4, future_environment(), &catalog, &environment_only)
        .unwrap();
    assert_eq!(
        fallback
            .implementations
            .get(&CapabilityId::BitBakeGetVar)
            .unwrap()
            .id,
        "bitbake.environment_lookup"
    );
    assert!(matches!(
        fallback
            .snapshot
            .capability(CapabilityId::BitBakeGetVar)
            .unwrap()
            .state,
        CapabilityState::AvailableWithLimitations { .. }
    ));
}
