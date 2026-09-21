use super::*;

#[test]
fn devtool_metadata_rejects_missing_or_relative_provider_identity() {
    let mut app = App::new(20, 4_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        file: Some("recipes-core/busybox.bb".into()),
        ..Recipe::default()
    });
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipeDevtoolStatus),
        None
    );
    assert_eq!(
        app.notification.as_deref(),
        Some("The selected recipe provider path is not absolute.")
    );
    assert!(app.devtool_status_loading.is_empty());
}
