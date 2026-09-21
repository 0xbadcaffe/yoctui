use super::*;

#[test]
fn build_environment_rejects_relative_profiles_and_preserves_unconfigured_state() {
    let mut app = App::new_unconfigured(16, 4096);
    let _ = update(
        &mut app,
        Action::ConfigureBuildEnvironment(BuildEnvironmentProfile {
            source_dir: PathBuf::from("poky"),
            build_dir: PathBuf::from("/workspace/build"),
            init_script: PathBuf::from("/workspace/poky/oe-init-build-env"),
        }),
    );
    assert_eq!(app.build_environment, BuildEnvironmentState::Unconfigured);
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("absolute"))
    );
}
