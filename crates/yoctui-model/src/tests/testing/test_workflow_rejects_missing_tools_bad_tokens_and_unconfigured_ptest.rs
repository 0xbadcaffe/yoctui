use super::*;

#[test]
fn test_workflow_rejects_missing_tools_bad_tokens_and_unconfigured_ptest() {
    let missing = TestCapability::default();
    let oe = TestLaunchDraft::new(
        TestFamily::OeSelftest,
        "qemux86-64".into(),
        "poky".into(),
        "image".into(),
    );
    assert_eq!(
        oe.preview(&missing),
        Err("test executable has not been inspected")
    );
    let bad = TestLaunchDraft::new(
        TestFamily::TestImage,
        "../machine".into(),
        "poky".into(),
        "image".into(),
    );
    assert!(bad.preview(&capability()).is_err());
    let ptest = TestLaunchDraft::new(
        TestFamily::Ptest,
        "qemux86-64".into(),
        "poky".into(),
        "image".into(),
    );
    assert!(ptest.preview(&missing).is_err());
}
