use super::*;

#[test]
fn compatibility_layers_old_surface_disables_only_absent_mutations_and_options() {
    let build = Path::new("/work/build");
    let executable = Path::new("/work/poky/bitbake/bin/bitbake-layers");
    let records = [
        (
            CapabilityId::BitBakeLayersShowLayers,
            BITBAKE_LAYERS_SHOW_IMPLEMENTATION,
            true,
        ),
        (
            CapabilityId::BitBakeLayersCreateLayer,
            BITBAKE_LAYERS_CREATE_IMPLEMENTATION,
            true,
        ),
        (
            CapabilityId::BitBakeLayersCreateAndAddLayer,
            BITBAKE_LAYERS_CREATE_ADD_IMPLEMENTATION,
            false,
        ),
        (
            CapabilityId::BitBakeLayersRemoveLayer,
            BITBAKE_LAYERS_REMOVE_IMPLEMENTATION,
            false,
        ),
    ];
    let authority = authority(build, executable, 12, &records);
    let planner = BitBakeLayersCommandPlanner::new(&authority, 12, build, executable).unwrap();
    planner
        .operation(&BitBakeLayersOperation::ShowLayers)
        .unwrap();
    planner
        .operation(&BitBakeLayersOperation::CreateLayer {
            directory: "/layers/meta-demo".into(),
            add: false,
            layer_id: None,
            priority: None,
            example_recipe: None,
            example_version: None,
        })
        .unwrap();
    assert!(matches!(
        planner.operation(&BitBakeLayersOperation::CreateLayer {
            directory: "/layers/meta-demo".into(),
            add: true,
            layer_id: None,
            priority: None,
            example_recipe: None,
            example_version: None,
        }),
        Err(BitBakeLayersCompatibilityError::Unavailable {
            capability: CapabilityId::BitBakeLayersCreateAndAddLayer,
            ..
        })
    ));
    assert!(matches!(
        planner.operation(&BitBakeLayersOperation::AddLayers {
            directories: vec!["/layers/meta-demo".into()]
        }),
        Err(BitBakeLayersCompatibilityError::CapabilityMissing {
            capability: CapabilityId::BitBakeLayersAddLayer
        })
    ));
}
