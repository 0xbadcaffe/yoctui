use super::*;

#[test]
fn raw_category_browser_pins_favorites_before_reference_order() {
    let catalog = builtin_raw_catalog();
    let categories = catalog.browser_categories();
    assert_eq!(categories.len(), crate::RAW_BUILTIN_CATEGORY_COUNT);
    assert_eq!(categories[0].kind, RawCategoryKind::Favorites);
    assert_eq!(categories[0].label, "Favorites");
    assert_eq!(categories[1].reference_heading, "1. Version and help");
    assert_eq!(
        categories.last().unwrap().reference_heading,
        "One-screen emergency reference"
    );

    let mut state = RawModeState::new(catalog);
    assert_eq!(state.category, Some(categories[0].id.clone()));
    reduce_raw_mode(
        &mut state,
        catalog,
        None,
        RawModeAction::SelectCategory { delta: 1 },
    );
    assert_eq!(state.category, Some(categories[1].id.clone()));
    reduce_raw_mode(
        &mut state,
        catalog,
        None,
        RawModeAction::SelectCategory { delta: isize::MAX },
    );
    assert_eq!(state.category, Some(categories.last().unwrap().id.clone()));
}
