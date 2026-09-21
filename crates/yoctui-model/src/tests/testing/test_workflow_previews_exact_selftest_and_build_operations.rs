use super::*;

#[test]
fn test_workflow_previews_exact_selftest_and_build_operations() {
    let mut oe = TestLaunchDraft::new(
        TestFamily::OeSelftest,
        "qemux86-64".into(),
        "poky".into(),
        "core-image-minimal".into(),
    );
    oe.scope = TestSelectorScope::Selected;
    oe.selector = "tinfoil.TinfoilTests.test_getvar".into();
    oe.parallelism = 8;
    let TestLaunchPreview::Selftest(request) = oe.preview(&capability()).unwrap() else {
        panic!("selftest preview");
    };
    assert_eq!(
        request.argv(),
        [
            PathBuf::from("/workspace/oe-selftest"),
            "-r".into(),
            "tinfoil.TinfoilTests.test_getvar".into(),
            "-j".into(),
            "8".into(),
        ]
    );

    for (family, task) in [
        (TestFamily::TestImage, "testimage"),
        (TestFamily::TestSdk, "testsdk"),
        (TestFamily::TestSdkExt, "testsdkext"),
        (TestFamily::Ptest, "testimage"),
    ] {
        let TestLaunchPreview::Build { request, .. } = TestLaunchDraft::new(
            family,
            "qemux86-64".into(),
            "poky".into(),
            "core-image-minimal".into(),
        )
        .preview(&capability())
        .unwrap() else {
            panic!("build preview");
        };
        assert_eq!(request.task.as_deref(), Some(task));
        assert_eq!(request.targets, ["core-image-minimal"]);
    }
}
