use super::*;

#[test]
fn sdk_workflow_build_preview_maps_exact_tasks() {
    for (action, task) in [
        (SdkBuildAction::Populate(SdkKind::Standard), "populate_sdk"),
        (
            SdkBuildAction::Populate(SdkKind::Extensible),
            "populate_sdk_ext",
        ),
        (SdkBuildAction::Test(SdkKind::Standard), "testsdk"),
        (SdkBuildAction::Test(SdkKind::Extensible), "testsdkext"),
    ] {
        let preview = SdkBuildPreview::new(
            "qemux86-64".into(),
            "poky".into(),
            "core-image-minimal".into(),
            action,
        )
        .unwrap();
        assert_eq!(preview.request.task.as_deref(), Some(task));
        assert_eq!(preview.request.targets, ["core-image-minimal"]);
        assert!(!preview.request.force);
    }
    assert!(
        SdkBuildPreview::new(
            "../machine".into(),
            "poky".into(),
            "image".into(),
            SdkBuildAction::Populate(SdkKind::Standard),
        )
        .is_err()
    );
}
