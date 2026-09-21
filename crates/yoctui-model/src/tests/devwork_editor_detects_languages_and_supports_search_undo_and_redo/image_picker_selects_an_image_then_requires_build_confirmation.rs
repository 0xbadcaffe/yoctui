use super::*;

#[test]
fn image_picker_selects_an_image_then_requires_build_confirmation() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::OpenImagePicker(vec!["core-image-base".into(), "core-image-minimal".into()]),
    );
    let _ = update(&mut app, Action::SelectImage { delta: 1 });
    let _ = update(&mut app, Action::ConfirmImagePicker);
    assert_eq!(app.build.target.as_deref(), Some("core-image-minimal"));
    let _ = update(&mut app, Action::BeginCurrentImageBuild);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        }))
    );
}
