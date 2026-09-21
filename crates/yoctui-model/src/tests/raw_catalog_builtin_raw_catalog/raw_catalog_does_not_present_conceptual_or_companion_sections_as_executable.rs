use super::*;

#[test]
fn raw_catalog_does_not_present_conceptual_or_companion_sections_as_executable() {
    let catalog = RawCatalog::builtin();
    assert_eq!(
        catalog
            .category(&RawCategoryId::new("section-29-quick-conceptual-reference").unwrap())
            .unwrap()
            .kind,
        RawCategoryKind::Conceptual
    );
    assert_eq!(
        catalog
            .category(&RawCategoryId::new("section-28-yocto-companion-commands").unwrap())
            .unwrap()
            .kind,
        RawCategoryKind::CompanionTools
    );
    assert_eq!(
        catalog
            .category(&RawCategoryId::new("favorites").unwrap())
            .unwrap()
            .kind,
        RawCategoryKind::Favorites
    );
}
