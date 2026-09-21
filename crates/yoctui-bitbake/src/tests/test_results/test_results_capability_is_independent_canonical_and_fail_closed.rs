use super::*;

#[test]
fn test_results_capability_is_independent_canonical_and_fail_closed() {
    let directory = TestDirectory::new("capability");
    let bin = directory.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let inspector = ResultToolCapabilityInspector::new(vec![bin.clone()]);
    assert_eq!(inspector.inspect(), ResultToolCapability::Missing);

    let tool = bin.join("resulttool");
    executable(&tool, "#!/bin/sh\nexit 0\n");
    assert_eq!(
        inspector.inspect(),
        ResultToolCapability::Available(tool.clone())
    );

    fs::remove_file(&tool).unwrap();
    let outside = directory.path().join("outside-resulttool");
    executable(&outside, "#!/bin/sh\nexit 0\n");
    symlink(&outside, &tool).unwrap();
    assert!(matches!(
        inspector.inspect(),
        ResultToolCapability::Failed(_)
    ));
    assert!(matches!(
        ResultToolCapabilityInspector::new(vec![directory.path().join("missing")]).inspect(),
        ResultToolCapability::Failed(_)
    ));
}
