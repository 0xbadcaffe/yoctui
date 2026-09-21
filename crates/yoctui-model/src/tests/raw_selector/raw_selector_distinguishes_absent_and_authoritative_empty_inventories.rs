use super::*;

#[test]
fn raw_selector_distinguishes_absent_and_authoritative_empty_inventories() {
    let absent = RawSelectorAuthority::project(RawSelectorSources::default());
    assert!(matches!(
        absent.recipes,
        RawSelectorInventory::Unavailable { .. }
    ));
    assert!(matches!(
        absent.targets,
        RawSelectorInventory::Unavailable { .. }
    ));

    let recipes = Vec::new();
    let images = Vec::new();
    let recent_targets = Vec::new();
    let metadata = RecipeMetadata {
        recipe: "busybox".into(),
        tasks: Some(Vec::new()),
        ..RecipeMetadata::default()
    };
    let empty = RawSelectorAuthority::project(RawSelectorSources {
        recipes: Some(&recipes),
        images: Some(&images),
        recent_targets: Some(&recent_targets),
        selected_recipe: Some("busybox"),
        recipe_metadata: Some(&metadata),
        multiconfig: Some(""),
        ..RawSelectorSources::default()
    });
    for inventory in [
        &empty.recipes,
        &empty.images,
        &empty.targets,
        &empty.tasks,
        &empty.multiconfigs,
    ] {
        assert_eq!(inventory.choices(), Some([].as_slice()));
    }
}
