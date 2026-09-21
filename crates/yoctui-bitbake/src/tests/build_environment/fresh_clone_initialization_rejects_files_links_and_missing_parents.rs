use super::*;

#[tokio::test]
async fn fresh_clone_initialization_rejects_files_links_and_missing_parents() {
    let root = std::env::temp_dir().join(format!("yoctui-fresh-invalid-{}", std::process::id()));
    let mut p = profile(&root);
    fs::create_dir_all(&p.source_dir).unwrap();
    crate::test_support::write_executable(&p.init_script, "#!/bin/bash\ntouch executed\n");
    let adapter = BuildEnvironmentAdapter::default();
    fs::write(&p.build_dir, "occupied").unwrap();
    assert!(adapter.initialize(p.clone()).await.is_err());
    fs::remove_file(&p.build_dir).unwrap();
    std::os::unix::fs::symlink(root.join("missing"), &p.build_dir).unwrap();
    assert!(adapter.initialize(p.clone()).await.is_err());
    fs::remove_file(&p.build_dir).unwrap();
    p.build_dir = root.join("missing/build");
    assert!(adapter.initialize(p.clone()).await.is_err());
    p.build_dir = root.join("../unsafe");
    assert!(adapter.initialize(p.clone()).await.is_err());
    assert!(!p.source_dir.join("executed").exists());
    fs::remove_dir_all(root).unwrap();
}
