use super::*;

#[test]
fn catalog_builtin_is_versioned_complete_unique_and_valid() {
    let catalog = CapabilityCatalog::builtin();
    catalog.validate().unwrap();
    assert_eq!(catalog.version, CAPABILITY_CATALOG_VERSION);
    assert_eq!(catalog.entries.len(), CapabilityId::ALL.len());
    for id in CapabilityId::ALL {
        let entry = catalog.entry(id).unwrap();
        assert!(!entry.probes.is_empty());
        assert_eq!(entry.id, id);
        assert_eq!(
            entry.unavailable_reason.requirement.as_deref(),
            Some(format!("Required capability: {}", id.as_str()).as_str())
        );
    }
}
