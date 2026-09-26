use super::*;

#[tokio::test]
async fn errors_resolved_cleanup_removes_only_selected_archive_identity() {
    let root = Root::new();
    save(&root.0, record(1)).unwrap();
    save(&root.0, record(2)).unwrap();

    let archive = remove(&root.0, "1".into()).await.unwrap();
    assert_eq!(archive.builds.len(), 1);
    assert_eq!(archive.builds[0].id, "2");
    assert_eq!(read(&root.0).unwrap(), archive);

    let error = remove(&root.0, "missing".into())
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("no longer exists"), "{error}");
    assert_eq!(read(&root.0).unwrap().builds[0].id, "2");
}
