use super::*;

#[test]
fn daemon_compatibility_discovers_directory_symlinks_and_preserves_tool_alias_name() {
    let fixture = RuntimeFixture::new();
    write_tool(&fixture.bin.join("devtool"), "echo fixture");
    write_tool(&fixture.bin.join("bitbake-diffsigs"), "echo fixture");
    std::os::unix::fs::symlink("bitbake-diffsigs", fixture.bin.join("bitbake-dumpsig")).unwrap();
    let alias = fixture.root.join("scripts");
    std::os::unix::fs::symlink(&fixture.bin, &alias).unwrap();
    let path = alias.to_str().unwrap();
    assert_eq!(
        discover_executable(path, "devtool"),
        Some(fixture.bin.join("devtool"))
    );
    assert_eq!(
        discover_executable(path, "bitbake-dumpsig"),
        Some(fixture.bin.join("bitbake-dumpsig"))
    );
}
