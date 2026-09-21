use super::*;

#[tokio::test]
async fn initializes_child_environment_without_mutating_parent() {
    let root = std::env::temp_dir().join(format!("yoctui-env-{}", std::process::id()));
    let p = profile(&root);
    fs::create_dir_all(&p.source_dir).unwrap();
    fs::create_dir_all(&p.build_dir).unwrap();
    fs::write(&p.init_script, "export YOCTUI_TEST=ok\n").unwrap();
    fs::set_permissions(&p.init_script, fs::Permissions::from_mode(0o755)).unwrap();
    let response = BuildEnvironmentAdapter::default()
        .initialize(p.clone())
        .await
        .unwrap();
    assert_eq!(response.environment.get("YOCTUI_TEST"), Some(&"ok".into()));
    assert_eq!(
        response.environment.get("BUILDDIR"),
        Some(&p.build_dir.display().to_string())
    );
    assert!(std::env::var_os("YOCTUI_TEST").is_none());
    let _ = fs::remove_dir_all(root);
}
