use super::*;

#[test]
fn setup_catalogue_reads_generated_wrapper_without_executing_it() {
    let dir = tempfile::tempdir().unwrap();
    let build = dir.path().join("demo/build");
    fs::create_dir_all(build.join("conf")).unwrap();
    fs::write(build.join("conf/local.conf"), "").unwrap();
    fs::write(build.join("conf/bblayers.conf"), "").unwrap();
    fs::write(build.join("init-build-env"), "exit 123\n").unwrap();
    let entries = scan(dir.path()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "demo");
    assert_eq!(entries[0].profile.build_dir, build);
    assert!(scan(&dir.path().join("missing")).unwrap().is_empty());
    assert!(scan(Path::new("relative")).is_err());
}

#[test]
#[cfg(unix)]
fn setup_launch_preserves_arguments_and_refuses_existing_or_dangling_destination() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let dir = tempfile::tempdir().unwrap();
    let tool = dir.path().join("bitbake-setup");
    fs::write(&tool, "#!/bin/sh\nexit 123\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    let top = dir.path().join("top space");
    let values = [
        tool.display().to_string(),
        top.display().to_string(),
        "demo".into(),
        "template $(false)".into(),
        "registry with spaces".into(),
    ];
    let plan = BitBakeSetupPlan::from_values(&values).unwrap();
    let launch = prepare(&plan).unwrap();
    assert_eq!(launch.program, tool);
    assert_eq!(launch.arguments.last().unwrap(), "template $(false)");
    assert!(launch.arguments.contains(&"registry with spaces".into()));
    assert!(
        !top.exists(),
        "preparation must not execute the tool/create files"
    );
    fs::create_dir(&top).unwrap();
    symlink(top.join("missing"), plan.destination()).unwrap();
    assert!(prepare(&plan).unwrap_err().contains("already exists"));
}
