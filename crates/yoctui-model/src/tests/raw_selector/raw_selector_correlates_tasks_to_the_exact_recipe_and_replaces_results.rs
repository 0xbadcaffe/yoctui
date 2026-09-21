use super::*;

#[test]
fn raw_selector_correlates_tasks_to_the_exact_recipe_and_replaces_results() {
    let alpha = RecipeMetadata {
        recipe: "alpha".into(),
        tasks: Some(vec!["do_build".into(), "do_compile".into()]),
        ..RecipeMetadata::default()
    };
    let current = RawSelectorAuthority::project(RawSelectorSources {
        selected_recipe: Some("alpha"),
        recipe_metadata: Some(&alpha),
        ..RawSelectorSources::default()
    });
    assert_eq!(current.tasks.choices().unwrap().len(), 2);

    let stale = RawSelectorAuthority::project(RawSelectorSources {
        selected_recipe: Some("beta"),
        recipe_metadata: Some(&alpha),
        ..RawSelectorSources::default()
    });
    assert!(matches!(
        stale.tasks,
        RawSelectorInventory::Unavailable { ref reason }
            if reason.contains("cannot populate")
    ));

    let beta = RecipeMetadata {
        recipe: "beta".into(),
        tasks: Some(vec!["do_install".into()]),
        ..RecipeMetadata::default()
    };
    let replacement = RawSelectorAuthority::project(RawSelectorSources {
        selected_recipe: Some("beta"),
        recipe_metadata: Some(&beta),
        ..RawSelectorSources::default()
    });
    assert_eq!(
        replacement.tasks.choices().unwrap()[0].identity,
        RawSelectorIdentity::Task {
            recipe: "beta".into(),
            task: "do_install".into()
        }
    );
}
