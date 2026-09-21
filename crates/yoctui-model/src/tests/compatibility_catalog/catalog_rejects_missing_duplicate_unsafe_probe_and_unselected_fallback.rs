use super::*;

#[test]
fn catalog_rejects_missing_duplicate_unsafe_probe_and_unselected_fallback() {
    let mut missing = CapabilityCatalog::builtin();
    missing.entries.pop();
    assert!(matches!(
        missing.validate(),
        Err(CapabilityCatalogError::Missing(_))
    ));

    let mut duplicate = CapabilityCatalog::builtin();
    duplicate.entries.push(duplicate.entries[0].clone());
    assert!(matches!(
        duplicate.validate(),
        Err(CapabilityCatalogError::Duplicate(_))
    ));

    let mut unsafe_probe = CapabilityCatalog::builtin();
    unsafe_probe.entries[0].probes = vec![CapabilityProbeSpec::CommandHelp {
        tool: CapabilityToolId::Devtool,
        subcommand: Some("bad subcommand".into()),
    }];
    assert_eq!(
        unsafe_probe.validate(),
        Err(CapabilityCatalogError::InvalidEntry(
            unsafe_probe.entries[0].id
        ))
    );

    let mut duplicate_probe = CapabilityCatalog::builtin();
    let repeated_probe = duplicate_probe.entries[0].probes[0].clone();
    duplicate_probe.entries[0].probes.push(repeated_probe);
    assert_eq!(
        duplicate_probe.validate(),
        Err(CapabilityCatalogError::InvalidEntry(
            duplicate_probe.entries[0].id
        ))
    );

    let mut bad_fallback = CapabilityCatalog::builtin();
    bad_fallback.entries[0].fallback = Some(FallbackImplementation {
        implementation: CapabilityImplementation {
            id: "fallback.test".into(),
            kind: CapabilityImplementationKind::Command,
        },
        selector: FallbackSelector::PositiveProbe { index: usize::MAX },
    });
    assert_eq!(
        bad_fallback.validate(),
        Err(CapabilityCatalogError::InvalidFallback(
            bad_fallback.entries[0].id
        ))
    );
}
