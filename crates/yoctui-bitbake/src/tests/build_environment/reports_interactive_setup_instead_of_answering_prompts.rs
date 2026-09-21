use super::*;

#[tokio::test]
async fn reports_interactive_setup_instead_of_answering_prompts() {
    let root = std::env::temp_dir().join(format!("yoctui-env-interactive-{}", std::process::id()));
    let p = profile(&root);
    fs::create_dir_all(&p.source_dir).unwrap();
    fs::create_dir_all(&p.build_dir).unwrap();
    fs::write(&p.init_script, "echo 'Continue? (y/n)' >&2; exit 1\n").unwrap();
    fs::set_permissions(&p.init_script, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        BuildEnvironmentAdapter::default().initialize(p).await,
        Err(BuildEnvironmentAdapterError::InteractiveRequired)
    ));
    let _ = fs::remove_dir_all(root);
}
