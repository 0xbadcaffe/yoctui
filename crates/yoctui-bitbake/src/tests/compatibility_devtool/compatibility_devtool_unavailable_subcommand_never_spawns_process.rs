use super::*;

#[tokio::test]
async fn compatibility_devtool_unavailable_subcommand_never_spawns_process() {
    use std::os::unix::fs::PermissionsExt;

    let root = std::env::temp_dir().join(format!(
        "yoctui-compatibility-devtool-no-spawn-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let executable = root.join("devtool");
    let marker = root.join("spawned");
    fs::write(
        &executable,
        format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let authority = authority(&root, &executable, 6, &[], &[CapabilityId::DevtoolUpgrade]);
    let planner = DevtoolCommandPlanner::new(&authority, 6, &root, &executable).unwrap();
    let planned = planner.operation(&DevtoolOperation::Upgrade {
        recipe: "busybox".into(),
    });
    assert!(planned.is_err());
    assert!(!marker.exists());
    fs::remove_dir_all(root).unwrap();
}
