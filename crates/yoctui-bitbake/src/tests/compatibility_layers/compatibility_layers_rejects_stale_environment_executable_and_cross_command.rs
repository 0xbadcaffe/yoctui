use super::*;

#[test]
fn compatibility_layers_rejects_stale_environment_executable_and_cross_command() {
    let build = Path::new("/work/build");
    let executable = Path::new("/work/poky/bitbake/bin/bitbake-layers");
    let authority = authority(
        build,
        executable,
        13,
        &[(
            CapabilityId::BitBakeLayersShowLayers,
            BITBAKE_LAYERS_SHOW_IMPLEMENTATION,
            true,
        )],
    );
    assert!(matches!(
        BitBakeLayersCommandPlanner::new(&authority, 12, build, executable),
        Err(BitBakeLayersCompatibilityError::StaleGeneration { .. })
    ));
    assert!(matches!(
        BitBakeLayersCommandPlanner::new(&authority, 13, Path::new("/other"), executable),
        Err(BitBakeLayersCompatibilityError::EnvironmentMismatch)
    ));
    assert!(matches!(
        BitBakeLayersCommandPlanner::new(
            &authority,
            13,
            build,
            Path::new("/usr/bin/bitbake-layers")
        ),
        Err(BitBakeLayersCompatibilityError::ExecutableMismatch)
    ));
    let planner = BitBakeLayersCommandPlanner::new(&authority, 13, build, executable).unwrap();
    assert!(matches!(
        planner.operation(&BitBakeLayersOperation::RemoveLayers {
            directories: vec!["/layers/meta-old".into()]
        }),
        Err(BitBakeLayersCompatibilityError::CapabilityMissing {
            capability: CapabilityId::BitBakeLayersRemoveLayer
        })
    ));
}
