use super::*;

#[test]
fn test_results_import_and_junit_require_exact_non_overwriting_paths() {
    assert!(TestResultImportRequest::new(1, vec!["relative".into()]).is_err());
    let request = TestResultImportRequest::new(2, vec!["/build/results".into()]).unwrap();
    assert_eq!(request.roots, [PathBuf::from("/build/results")]);

    let result = result_identity("candidate", "candidate");
    let valid = TestJunitDestinationInspection {
        requested: "/exports/candidate.xml".into(),
        canonical_parent: Some("/exports".into()),
        parent_exists: true,
        parent_is_directory: true,
        destination_exists: false,
        destination_is_symlink: false,
    };
    let export = TestJunitExportRequest::new(1, result.clone(), &valid).unwrap();
    let preview = TestJunitExportPreview::new("/workspace/resulttool".into(), export).unwrap();
    assert_eq!(
        preview.argv,
        [
            PathBuf::from("/workspace/resulttool"),
            "junit".into(),
            result.path,
            "-j".into(),
            "/exports/candidate.xml".into(),
        ]
    );

    for invalid in [
        TestJunitDestinationInspection {
            requested: "relative.xml".into(),
            canonical_parent: Some("/exports".into()),
            ..valid.clone()
        },
        TestJunitDestinationInspection {
            destination_exists: true,
            ..valid.clone()
        },
        TestJunitDestinationInspection {
            canonical_parent: Some("/other".into()),
            ..valid.clone()
        },
    ] {
        assert!(invalid.validated_destination().is_err());
    }
}
