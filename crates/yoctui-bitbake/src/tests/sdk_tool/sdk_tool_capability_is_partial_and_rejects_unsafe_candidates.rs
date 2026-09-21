use super::*;

#[test]
fn sdk_tool_capability_is_partial_and_rejects_unsafe_candidates() {
    let directory = TestDirectory::new("capability");
    let workspace = directory.path().join("workspace");
    let scripts = workspace.join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    executable(&scripts.join("oe-publish-sdk"), "#!/bin/sh\nexit 0\n");
    let inspector = SdkToolCapabilityInspector::new(vec![workspace.clone()]);
    assert!(matches!(
        inspector.inspect(),
        SdkToolCapability::Available {
            publish: Some(_),
            find_sysroot: None,
            run_native: None,
        }
    ));

    let outside = directory.path().join("outside");
    executable(&outside, "#!/bin/sh\nexit 0\n");
    symlink(&outside, scripts.join("oe-run-native")).unwrap();
    assert!(matches!(
        inspector.inspect(),
        SdkToolCapability::Failed { .. }
    ));
    assert!(matches!(
        SdkToolCapabilityInspector::new(vec![directory.path().join("missing")]).inspect(),
        SdkToolCapability::Failed { .. }
    ));
}
