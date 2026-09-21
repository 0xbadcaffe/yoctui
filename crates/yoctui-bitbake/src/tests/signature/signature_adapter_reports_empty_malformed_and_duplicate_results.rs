use super::*;

#[tokio::test]
async fn signature_adapter_reports_empty_malformed_and_duplicate_results() {
    let directory = TestDirectory::new("malformed");
    let dump = directory.path().join("dump");
    let diff = directory.path().join("diff");
    write_executable(&dump, "#!/bin/sh\nprintf 'nonsense\\n'\n");
    write_executable(&diff, "#!/bin/sh\nexit 0\n");
    let adapter = test_adapter(directory.path(), dump.clone(), diff);
    assert!(adapter.dump(target()).await.unwrap().records.is_empty());

    signature_path(directory.path(), "aaa");
    assert!(matches!(
        adapter.dump(target()).await,
        Err(SignatureAdapterError::Malformed(_))
    ));

    let identity = SignatureIdentity {
        target: target(),
        hash: Some("aaa".into()),
        path: Some(
            directory
                .path()
                .join("tmp/stamps/qemux86_64/busybox/1.0.do_compile.sigdata.aaa"),
        ),
    };
    let (left, _) = parse_signature_dump(&identity, &fixture("aaa")).unwrap();
    let (records, report) = normalize_signature_records(&target(), vec![left.clone(), left], 8);
    assert_eq!(records.len(), 1);
    assert_eq!(report.duplicate_records, 1);
}
