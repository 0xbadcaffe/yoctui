use super::*;

#[test]
fn raw_selector_retains_exact_recipe_and_typed_inventory_identities() {
    let recipes = vec![
        Recipe {
            name: "busybox".into(),
            file: Some("/layers/meta/recipes-core/busybox/busybox.bb".into()),
            ..Recipe::default()
        },
        Recipe {
            name: "busybox".into(),
            file: Some("/workspace/recipes/busybox.bb".into()),
            ..Recipe::default()
        },
    ];
    let images = vec![
        "core-image-minimal".into(),
        "core-image-full-cmdline".into(),
    ];
    let recent_targets = vec!["busybox".into(), "virtual/kernel".into(), "busybox".into()];
    let authority = RawSelectorAuthority::project(RawSelectorSources {
        recipes: Some(&recipes),
        images: Some(&images),
        current_target: Some("core-image-minimal"),
        recent_targets: Some(&recent_targets),
        multiconfig: Some("lib32 board1 lib32"),
        ..RawSelectorSources::default()
    });

    assert_eq!(authority.recipes.choices().unwrap().len(), 2);
    assert_ne!(
        authority.recipes.choices().unwrap()[0].identity,
        authority.recipes.choices().unwrap()[1].identity
    );
    assert_eq!(authority.images.choices().unwrap().len(), 2);
    assert_eq!(authority.targets.choices().unwrap().len(), 3);
    assert_eq!(authority.multiconfigs.choices().unwrap().len(), 2);
    assert_eq!(
        authority.targets.choices().unwrap()[1].value,
        RawParameterValue::Target("busybox".into())
    );
}
