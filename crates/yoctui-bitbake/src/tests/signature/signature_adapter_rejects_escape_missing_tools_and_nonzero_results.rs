use super::*;

#[tokio::test]
async fn signature_adapter_rejects_escape_missing_tools_and_nonzero_results() {
    let directory = TestDirectory::new("errors");
    let outside_directory = TestDirectory::new("outside");
    let outside = outside_directory.path().join("outside.sigdata.aaa");
    fs::write(&outside, "{}").unwrap();
    let identity = SignatureIdentity {
        target: target(),
        hash: Some("aaa".into()),
        path: Some(outside.clone()),
    };
    let request = SignatureComparisonRequest {
        left: identity,
        right: SignatureIdentity {
            target: target(),
            hash: Some("bbb".into()),
            path: Some(outside),
        },
    };
    let adapter = test_adapter(
        directory.path(),
        directory.path().join("missing"),
        directory.path().join("missing"),
    );
    assert!(matches!(
        adapter.compare(request).await,
        Err(SignatureAdapterError::PathEscape(_))
    ));

    let path = signature_path(directory.path(), "aaa");
    assert!(matches!(
        adapter.dump(target()).await,
        Err(SignatureAdapterError::MissingTool(_))
    ));
    let failure = directory.path().join("failure");
    write_executable(&failure, "#!/bin/sh\nprintf 'bad input\\n' >&2\nexit 7\n");
    let error = test_adapter(directory.path(), failure, directory.path().join("unused"))
        .dump(target())
        .await
        .unwrap_err();
    assert_eq!(
        error,
        SignatureAdapterError::NonZero {
            exit_code: Some(7),
            message: "bad input".into()
        }
    );
    assert!(path.exists());
}
