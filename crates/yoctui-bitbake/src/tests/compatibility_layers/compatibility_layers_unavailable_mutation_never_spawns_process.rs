use super::*;

#[test]
fn compatibility_layers_unavailable_mutation_never_spawns_process() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!(
        "yoctui-compatibility-layers-no-spawn-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let executable = root.join("bitbake-layers");
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
        14,
        &[(
            CapabilityId::BitBakeLayersRemoveLayer,
            BITBAKE_LAYERS_REMOVE_IMPLEMENTATION,
            false,
        )],
    );
    let planner = BitBakeLayersCommandPlanner::new(&authority, 14, &root, &executable).unwrap();
    assert!(
        planner
            .operation(&BitBakeLayersOperation::RemoveLayers {
                directories: vec!["/layers/meta-old".into()]
            })
            .is_err()
    );
    assert!(!marker.exists());
    fs::remove_dir_all(root).unwrap();
}
