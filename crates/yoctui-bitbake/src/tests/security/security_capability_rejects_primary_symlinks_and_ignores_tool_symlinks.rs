use super::*;

#[test]
fn security_capability_rejects_primary_symlinks_and_ignores_tool_symlinks() {
    use std::os::unix::fs::symlink;

    let (directory, mut input) = fixture(&["do_cve_check"]);
    let real_build = input.build_directory.clone();
    let linked_build = directory.path().join("linked-build");
    symlink(&real_build, &linked_build).unwrap();
    input.build_directory = linked_build;
    assert!(matches!(
        SecurityCapabilityInspector::new(input).inspect(),
        Err(SecurityCapabilityError::UnsafeBuildDirectory(_))
    ));

    let (_directory, input) = fixture(&["do_cve_check"]);
    let mapper = input.path_directories[0].join("cve-check-map-pkgs");
    fs::remove_file(&mapper).unwrap();
    symlink("/bin/sh", &mapper).unwrap();
    let snapshot = SecurityCapabilityInspector::new(input).inspect().unwrap();
    assert!(snapshot.mapper.is_none());
    assert!(
        snapshot
            .limitations
            .iter()
            .any(|value| value.contains("unsafe Security executable"))
    );
}
