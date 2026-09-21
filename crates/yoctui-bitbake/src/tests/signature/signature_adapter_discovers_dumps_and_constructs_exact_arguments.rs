use super::*;

#[tokio::test]
async fn signature_adapter_discovers_dumps_and_constructs_exact_arguments() {
    let directory = TestDirectory::new("dump");
    let path = signature_path(directory.path(), "aaa");
    let dump = directory.path().join("dump");
    let diff = directory.path().join("diff");
    write_executable(
        &dump,
        &format!(
            "#!/bin/sh\n[ \"$#\" -eq 1 ] || exit 8\n[ \"$1\" = \"{}\" ] || exit 9\nprintf '%s' '{}'\n",
            path.display(),
            fixture("aaa").replace('\'', "'\\''")
        ),
    );
    write_executable(&diff, "#!/bin/sh\nexit 0\n");
    let response = test_adapter(directory.path(), dump, diff)
        .dump(target())
        .await
        .unwrap();
    assert_eq!(response.records.len(), 1);
    assert_eq!(response.records[0].identity.path.as_ref(), Some(&path));
    assert!(response.limitations.is_empty());
}
