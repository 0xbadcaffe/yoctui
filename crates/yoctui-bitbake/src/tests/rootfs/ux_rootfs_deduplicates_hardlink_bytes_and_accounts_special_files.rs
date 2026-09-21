use super::*;

#[tokio::test]
async fn ux_rootfs_deduplicates_hardlink_bytes_and_accounts_special_files() {
    use std::os::unix::net::UnixListener;

    let (build, request, sources) = fixture();
    let root = sources.image_rootfs.as_ref().unwrap();
    fs::hard_link(
        root.join("usr/bin/busybox"),
        root.join("usr/bin/busybox.link"),
    )
    .unwrap();
    let socket = root.join("run.sock");
    let _listener = UnixListener::bind(&socket).unwrap();
    let response = RootfsCompositionAdapter::new(build.clone(), sources, 4)
        .scan(request)
        .await
        .unwrap();
    let tree = response.composition.filesystem_tree().unwrap();
    let hardlink_bytes = tree
        .entries
        .iter()
        .filter(|entry| entry.identity.0.to_string_lossy().contains("busybox"))
        .map(|entry| entry.size_bytes)
        .sum::<u64>();
    assert_eq!(hardlink_bytes, 7);
    assert!(tree.entries.iter().any(|entry| {
        entry.identity.0 == Path::new("/run.sock") && entry.kind == RootfsEntryKind::Other
    }));
    drop(_listener);
    fs::remove_dir_all(build).unwrap();
}
