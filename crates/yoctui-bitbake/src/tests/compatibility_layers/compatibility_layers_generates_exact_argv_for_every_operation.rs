use super::*;

#[test]
fn compatibility_layers_generates_exact_argv_for_every_operation() {
    let build = Path::new("/work/build");
    let executable = Path::new("/work/poky/bitbake/bin/bitbake-layers");
    let authority = authority(build, executable, 11, &all_records());
    let planner = BitBakeLayersCommandPlanner::new(&authority, 11, build, executable).unwrap();
    let cases = [
        (BitBakeLayersOperation::ShowLayers, vec!["show-layers"]),
        (
            BitBakeLayersOperation::ShowRecipes {
                pattern: Some("linux-*".into()),
            },
            vec!["show-recipes", "linux-*"],
        ),
        (
            BitBakeLayersOperation::ShowOverlayed,
            vec!["show-overlayed"],
        ),
        (
            BitBakeLayersOperation::CreateLayer {
                directory: "/layers/meta-demo".into(),
                add: false,
            },
            vec!["create-layer", "/layers/meta-demo"],
        ),
        (
            BitBakeLayersOperation::CreateLayer {
                directory: "/layers/meta-demo".into(),
                add: true,
            },
            vec!["create-layer", "--add-layer", "/layers/meta-demo"],
        ),
        (
            BitBakeLayersOperation::AddLayers {
                directories: vec!["/layers/meta-one".into(), "/layers/meta-two".into()],
            },
            vec!["add-layer", "/layers/meta-one", "/layers/meta-two"],
        ),
        (
            BitBakeLayersOperation::RemoveLayers {
                directories: vec!["/layers/meta-old".into()],
            },
            vec!["remove-layer", "/layers/meta-old"],
        ),
    ];
    for (operation, expected) in cases {
        let command = planner.operation(&operation).unwrap();
        assert_eq!(command.executable(), executable);
        assert_eq!(command.arguments(), expected);
        assert_eq!(command.generation(), 11);
    }
}
