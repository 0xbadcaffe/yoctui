use super::*;

#[tokio::test]
async fn poky_clone_requires_empty_destination_and_uses_reviewed_vectors() {
    let root = std::env::temp_dir().join(format!("yoctui-clone-{}", std::process::id()));
    let bin = root.join("git-fixture");
    let destination = root.join("poky");
    fs::create_dir_all(&root).unwrap();
    crate::test_support::write_executable(
        &bin,
        "#!/bin/sh\nif [ \"$2\" = \"--no-checkout\" ]; then mkdir -p \"$4\"; else mkdir -p \"$3\"; fi\n",
    );
    let request = BuildEnvironmentCloneRequest {
        repository: "https://example.invalid/poky".into(),
        destination: destination.clone(),
        revision: Some("scarthgap".into()),
    };
    let adapter = BuildEnvironmentAdapter::default().with_git_program(bin);
    let preview = adapter.preview_clone(&request).unwrap();
    assert_eq!(preview.clone_argv[1], "--no-checkout");
    adapter.clone_poky(request).await.unwrap();
    assert!(destination.is_dir());
    let _ = fs::remove_dir_all(root);
}
