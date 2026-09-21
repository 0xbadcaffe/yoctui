use super::*;

#[test]
fn daemon_compatibility_directory_resolution_keeps_unsafe_tools_rejected() {
    let fixture = RuntimeFixture::new();
    let alias = fixture.root.join("scripts");
    std::os::unix::fs::symlink(&fixture.bin, &alias).unwrap();
    write_tool(&fixture.root.join("outside"), "echo fixture");
    std::os::unix::fs::symlink("../outside", fixture.bin.join("escape")).unwrap();
    std::os::unix::fs::symlink(fixture.root.join("outside"), fixture.bin.join("absolute")).unwrap();
    std::os::unix::fs::symlink("missing", fixture.bin.join("dangling")).unwrap();
    fs::write(fixture.bin.join("non-executable"), "not a tool").unwrap();
    fs::create_dir(fixture.bin.join("directory")).unwrap();
    for name in [
        "escape",
        "absolute",
        "dangling",
        "non-executable",
        "directory",
        "missing",
    ] {
        assert!(
            discover_executable(alias.to_str().unwrap(), name).is_none(),
            "{name}"
        );
    }
    assert!(discover_executable(".", "bitbake").is_none());
    assert!(discover_executable("", "bitbake").is_none());
    std::os::unix::fs::symlink("missing", fixture.root.join("dangling-directory")).unwrap();
    assert!(
        discover_executable(
            fixture.root.join("dangling-directory").to_str().unwrap(),
            "bitbake"
        )
        .is_none()
    );
}
