use super::*;

#[test]
fn maintenance_release_capability_keeps_optional_build_compare_distinct() {
    let fixture = TestDirectory::new("capability");
    prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
    executable(&fixture.join("tools/build-compare"), "#!/bin/sh\nexit 0\n");
    let snapshot = inspect(&fixture);
    for tool in [
        MaintenanceTool::LockedSignatureCache,
        MaintenanceTool::BuildHistoryDiff,
        MaintenanceTool::GitArchive,
    ] {
        assert!(snapshot.supports(tool));
    }
    assert!(!snapshot.supports(MaintenanceTool::BuildCompare));
    assert!(
        snapshot
            .limitations
            .iter()
            .any(|line| line.contains("not the buildhistory-diff interface"))
    );
    assert!(matches!(
        build_compare_command(
            MaintenanceSessionId(1),
            &snapshot,
            comparison_request(&fixture)
        ),
        Err(MaintenanceReleaseAdapterError::Unavailable(_))
    ));

    #[cfg(unix)]
    {
        let unsafe_fixture = TestDirectory::new("unsafe-capability");
        prepare_fixture(&unsafe_fixture, "#!/bin/sh\nexit 0\n");
        fs::remove_file(unsafe_fixture.join("tools/gen-lockedsig-cache")).unwrap();
        let real = unsafe_fixture.join("real-tool");
        executable(&real, "#!/bin/sh\nexit 0\n");
        symlink(&real, unsafe_fixture.join("tools/gen-lockedsig-cache")).unwrap();
        let snapshot = inspect(&unsafe_fixture);
        assert!(!snapshot.supports(MaintenanceTool::LockedSignatureCache));
        assert!(
            snapshot
                .limitations
                .iter()
                .any(|line| line.contains("unsafe executable"))
        );
    }
}
