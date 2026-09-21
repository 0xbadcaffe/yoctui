use super::*;

#[tokio::test]
async fn fresh_clone_initialization_creates_reviewed_build_directory() {
    let root = std::env::temp_dir().join(format!("yoctui-fresh-clone-{}", std::process::id()));
    let p = profile(&root);
    fs::create_dir_all(&p.source_dir).unwrap();
    crate::test_support::write_executable(
        &p.init_script,
        "#!/bin/bash\nmkdir -p \"$1/conf\"\nexport BUILDDIR=\"$1\"\n",
    );
    assert!(!p.build_dir.exists());
    let adapter = BuildEnvironmentAdapter::default();
    adapter.validate(&p).unwrap();
    assert!(
        !p.build_dir.exists(),
        "validation must not create directories"
    );
    let response = adapter.initialize(p.clone()).await.unwrap();
    assert!(p.build_dir.join("conf").is_dir());
    assert_eq!(
        response.environment.get("BUILDDIR"),
        Some(&p.build_dir.display().to_string())
    );
    fs::remove_dir_all(root).unwrap();
}
