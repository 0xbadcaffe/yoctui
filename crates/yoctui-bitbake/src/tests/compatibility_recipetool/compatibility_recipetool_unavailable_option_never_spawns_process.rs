use super::*;

#[test]
fn compatibility_recipetool_unavailable_option_never_spawns_process() {
    use std::os::unix::fs::PermissionsExt;

    let root = std::env::temp_dir().join(format!(
        "yoctui-compatibility-recipetool-no-spawn-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let executable = root.join("recipetool");
    let marker = root.join("spawned");
    fs::write(
        &executable,
        format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let authority = authority(
        &root,
        &executable,
        10,
        &[(
            CapabilityId::RecipetoolCreate,
            RECIPETOOL_CREATE_IMPLEMENTATION,
        )],
        &[CapabilityId::RecipetoolCreateOutfile],
    );
    let planner = RecipetoolCommandPlanner::new(&authority, 10, &root, &executable).unwrap();
    assert!(planner.operation(&create()).is_err());
    assert!(!marker.exists());
    fs::remove_dir_all(root).unwrap();
}
