use super::*;

#[test]
fn compatibility_older_release_preserves_core_selects_fallback_and_disables_newer_feature() {
    let catalog = CapabilityCatalog::builtin();
    let mut environment = future_environment();
    environment.bitbake_version =
        AuthoritativeValue::detected("1.52.0".into(), IdentityAuthority::BitBakeVersionProbe);
    environment.oe_core = AuthoritativeValue::detected(
        ReleaseIdentity {
            name: Some("honister".into()),
            version: Some("3.4".into()),
        },
        IdentityAuthority::ReleaseMetadata,
    );
    let observations = BTreeMap::from([
        (
            CapabilityId::BitBakeWorkspaceInspection,
            vec![observation(CapabilityProbeStatus::Positive, "workspace")],
        ),
        (
            CapabilityId::DevtoolUpgrade,
            vec![observation(CapabilityProbeStatus::Negative, "upgrade")],
        ),
        (
            CapabilityId::BitBakeNativeEvents,
            vec![observation(
                CapabilityProbeStatus::Inconclusive,
                "native events backend unprobeable",
            )],
        ),
    ]);
    let resolved = CapabilityResolver::default()
        .resolve_snapshot(2, environment, &catalog, &observations)
        .unwrap();
    assert!(
        resolved
            .snapshot
            .allows(CapabilityId::BitBakeWorkspaceInspection)
    );
    assert!(resolved.snapshot.allows(CapabilityId::BitBakeNativeEvents));
    assert!(!resolved.snapshot.allows(CapabilityId::DevtoolUpgrade));
    assert_eq!(
        resolved
            .implementations
            .get(&CapabilityId::BitBakeNativeEvents)
            .unwrap()
            .id,
        "tinfoil.adapter.legacy"
    );
    assert!(matches!(
        resolved
            .snapshot
            .capability(CapabilityId::BitBakeNativeEvents)
            .unwrap()
            .state,
        CapabilityState::AvailableWithLimitations { .. }
    ));
    assert!(matches!(
        resolved
            .snapshot
            .capability(CapabilityId::DevtoolUpgrade)
            .unwrap()
            .state,
        CapabilityState::Unavailable { .. }
    ));
    assert_eq!(
        resolved.snapshot.capabilities.len(),
        CapabilityId::ALL.len()
    );
}
