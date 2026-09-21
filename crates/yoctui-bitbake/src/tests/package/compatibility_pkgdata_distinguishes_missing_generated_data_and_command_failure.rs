use super::*;

#[tokio::test]
async fn compatibility_pkgdata_distinguishes_missing_generated_data_and_command_failure() {
    let directory = TestDirectory::new("failures");
    let build_dir = directory.path().join("build");
    fs::create_dir_all(&build_dir).unwrap();
    let tool = directory.path().join("oe-pkgdata-util");
    fs::write(&tool, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    let authority = compatibility(&build_dir, &tool);
    let missing =
        PackageDataAdapter::with_paths(build_dir.clone(), tool, build_dir.join("tmp/pkgdata"))
            .with_compatibility(authority, 1)
            .unwrap()
            .inventory(inventory_request())
            .await
            .unwrap_err();
    assert!(matches!(
        missing,
        PackageDataAdapterError::MissingPkgdata(_)
    ));

    let script = "#!/bin/sh\nprintf 'broken metadata\\n' >&2\nexit 17\n";
    let (_fixture, adapter, _log) = fixture("nonzero", script);
    assert_eq!(
        adapter.inventory(inventory_request()).await.unwrap_err(),
        PackageDataAdapterError::NonZero {
            exit_code: Some(17),
            message: "broken metadata".into(),
        }
    );
    assert!(matches!(
        adapter
            .inventory(PackageInventoryRequest { generation: 0 })
            .await,
        Err(PackageDataAdapterError::InvalidRequest(_))
    ));
    assert!(matches!(
        adapter
            .detail(PackageDetailRequest {
                identity: PackageIdentity::new("bad package"),
                generation: 1,
            })
            .await,
        Err(PackageDataAdapterError::InvalidRequest(_))
    ));
}
