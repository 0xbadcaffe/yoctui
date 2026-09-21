use super::*;

#[test]
fn fresh_clone_preview_rejects_dangling_destination() {
    let root = std::env::temp_dir().join(format!("yoctui-clone-link-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let destination = root.join("poky");
    std::os::unix::fs::symlink(root.join("missing"), &destination).unwrap();
    let request = BuildEnvironmentCloneRequest {
        repository: "https://example.invalid/poky".into(),
        destination,
        revision: None,
    };
    assert!(matches!(
        BuildEnvironmentAdapter::default().preview_clone(&request),
        Err(BuildEnvironmentAdapterError::UnsafePath(_))
    ));
    fs::remove_dir_all(root).unwrap();
}
