use super::*;

#[test]
fn build_environment_clone_draft_does_not_guess_machine_paths() {
    let mut app = App::new_unconfigured(8, 512);
    assert!(update(&mut app, Action::OpenBuildEnvironmentCloneEditor).is_none());
    let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog() else {
        panic!("clone editor was not opened");
    };
    let fields: toml::Table = toml::from_str(&editor.text).unwrap();
    for key in ["repository", "destination", "revision", "build"] {
        assert_eq!(fields[key].as_str(), Some(""));
    }
    assert!(update(&mut app, Action::ReviewBuildEnvironmentClone).is_none());
    assert!(!matches!(
        app.active_dialog(),
        Some(Dialog::BuildEnvironmentCloneReview(_))
    ));
}
