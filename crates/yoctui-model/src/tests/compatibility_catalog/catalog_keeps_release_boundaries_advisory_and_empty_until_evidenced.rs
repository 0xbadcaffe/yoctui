use super::*;

#[test]
fn catalog_keeps_release_boundaries_advisory_and_empty_until_evidenced() {
    let catalog = CapabilityCatalog::builtin();
    assert!(
        catalog
            .entries
            .iter()
            .all(|entry| entry.known_release_boundaries.is_empty())
    );
    assert!(
        catalog
            .entries
            .iter()
            .all(|entry| !entry.label.contains("Yocto 5"))
    );
}
