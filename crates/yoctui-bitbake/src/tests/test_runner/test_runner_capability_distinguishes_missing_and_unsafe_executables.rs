use super::*;

#[test]
fn test_runner_capability_distinguishes_missing_and_unsafe_executables() {
    let directory = TestDirectory::new("capability");
    let bin = directory.path().join("bin");
    fs::create_dir(&bin).unwrap();
    executable(&bin.join("oe-selftest"), "#!/bin/sh\nexit 0\n");
    let inspector =
        TestRunnerCapabilityInspector::new(vec![bin.clone()], PtestCapability::Configured);
    assert!(matches!(
        inspector.inspect(),
        TestCapability {
            oe_selftest: TestExecutableCapability::Available(_),
            bitbake_selftest: TestExecutableCapability::Missing,
            ptest: PtestCapability::Configured,
        }
    ));

    let outside = directory.path().join("outside");
    executable(&outside, "#!/bin/sh\nexit 0\n");
    symlink(&outside, bin.join("bitbake-selftest")).unwrap();
    assert!(matches!(
        inspector.inspect().bitbake_selftest,
        TestExecutableCapability::Failed(_)
    ));
    assert!(matches!(
        TestRunnerCapabilityInspector::new(
            vec![directory.path().join("missing")],
            PtestCapability::NotInspected
        )
        .inspect()
        .oe_selftest,
        TestExecutableCapability::Failed(_)
    ));
}
