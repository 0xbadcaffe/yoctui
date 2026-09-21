use super::*;

#[test]
fn catalog_records_safe_explicit_fallback_selectors() {
    let catalog = CapabilityCatalog::builtin();
    let graph = catalog.entry(CapabilityId::BitBakeDependencyGraph).unwrap();
    assert!(matches!(
        graph.fallback.as_ref().map(|fallback| &fallback.selector),
        Some(FallbackSelector::AvailableCapability {
            id: CapabilityId::BitBakeGraphGeneration
        })
    ));
    let getvar = catalog.entry(CapabilityId::BitBakeGetVar).unwrap();
    assert_eq!(getvar.required_tools, vec![CapabilityToolId::BitBakeGetVar]);
    assert_eq!(getvar.required_commands.len(), 1);
    assert_eq!(
        getvar.required_commands[0].options,
        vec!["--value", "--recipe"]
    );
    assert_eq!(getvar.preferred.id, "bitbake_getvar.argv");
    assert!(matches!(
        getvar.fallback.as_ref().map(|fallback| &fallback.selector),
        Some(FallbackSelector::AvailableCapability {
            id: CapabilityId::BitBakeEnvironmentDump
        })
    ));
}
