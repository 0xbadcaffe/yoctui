use super::*;

#[test]
fn recipe_refresh_rebuilds_authoritative_image_inventory() {
    let mut app = App::new(16, 4096);
    let _ = update(
        &mut app,
        Action::RecipesLoaded(vec![
            Recipe {
                name: "base-files".into(),
                ..Recipe::default()
            },
            Recipe {
                name: "core-image-minimal".into(),
                ..Recipe::default()
            },
        ]),
    );
    assert_eq!(app.available_images, vec!["core-image-minimal"]);
}
