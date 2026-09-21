use super::*;

#[test]
fn compatibility_dynamic_model_unavailable_effect_is_not_emitted_or_partially_applied() {
    let mut app = App::new(10, 1_000);
    let request = BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: None,
        force: false,
    };
    app.dialogs
        .push_front(Dialog::RecipeTaskConfirmation(request));
    app.focus = crate::FocusTarget::Dialog;
    let before_build = app.build.clone();

    assert!(update_with_workspace_authority(&mut app, crate::Action::ConfirmRecipeTask).is_none());
    assert_eq!(app.build, before_build);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(_))
    ));
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("current environment capability snapshot")
    );
}
