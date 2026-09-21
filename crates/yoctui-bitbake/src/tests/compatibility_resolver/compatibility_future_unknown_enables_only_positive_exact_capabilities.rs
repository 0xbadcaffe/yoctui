use super::*;

#[test]
fn compatibility_future_unknown_enables_only_positive_exact_capabilities() {
    let catalog = CapabilityCatalog::builtin();
    let observations = BTreeMap::from([
        (
            CapabilityId::DevtoolUpgrade,
            complete_observations(
                &catalog,
                CapabilityId::DevtoolUpgrade,
                CapabilityProbeStatus::Positive,
                "upgrade",
            ),
        ),
        (
            CapabilityId::ResultTool,
            vec![observation(CapabilityProbeStatus::Negative, "resulttool")],
        ),
        (
            CapabilityId::WicCreate,
            vec![
                observation(CapabilityProbeStatus::Positive, "wic create"),
                observation(CapabilityProbeStatus::Negative, "wic option"),
            ],
        ),
    ]);
    let resolved = CapabilityResolver::default()
        .resolve_snapshot(1, future_environment(), &catalog, &observations)
        .unwrap();
    assert!(resolved.snapshot.allows(CapabilityId::DevtoolUpgrade));
    assert!(!resolved.snapshot.allows(CapabilityId::ResultTool));
    assert!(!resolved.snapshot.allows(CapabilityId::WicCreate));
    assert!(!resolved.snapshot.allows(CapabilityId::RunQemu));
    assert!(matches!(
        resolved
            .snapshot
            .capability(CapabilityId::ResultTool)
            .unwrap()
            .state,
        CapabilityState::Unavailable { .. }
    ));
    assert!(matches!(
        resolved
            .snapshot
            .capability(CapabilityId::WicCreate)
            .unwrap()
            .state,
        CapabilityState::Unknown { .. }
    ));
    assert!(matches!(
        resolved
            .snapshot
            .capability(CapabilityId::RunQemu)
            .unwrap()
            .state,
        CapabilityState::Unknown { .. }
    ));
    assert_eq!(
        resolved.snapshot.capabilities.len(),
        CapabilityId::ALL.len()
    );
    assert_eq!(
        resolved
            .snapshot
            .environment
            .oe_core
            .value()
            .unwrap()
            .name
            .as_deref(),
        Some("future-series")
    );
}
