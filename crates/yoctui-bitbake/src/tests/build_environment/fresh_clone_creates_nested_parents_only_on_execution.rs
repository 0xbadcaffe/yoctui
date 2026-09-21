use super::*;

#[tokio::test]
async fn fresh_clone_creates_nested_parents_only_on_execution() {
    let root = std::env::temp_dir().join(format!("yoctui-clone-nested-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let bin = root.join("git");
    crate::test_support::write_executable(&bin, "#!/bin/sh\nmkdir -p \"$3\"\n");
    let request = BuildEnvironmentCloneRequest {
        repository: "https://example.invalid/poky".into(),
        destination: root.join("new/parent/poky"),
        revision: None,
    };
    let adapter = BuildEnvironmentAdapter::default().with_git_program(bin);
    adapter.preview_clone(&request).unwrap();
    assert!(!root.join("new").exists());
    adapter.clone_poky(request.clone()).await.unwrap();
    assert!(request.destination.is_dir());
    fs::remove_dir_all(root).unwrap();
}
