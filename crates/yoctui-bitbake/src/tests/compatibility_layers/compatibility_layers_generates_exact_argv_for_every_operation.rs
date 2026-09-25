use super::*;

#[test]
fn compatibility_layers_generates_exact_argv_for_every_operation() {
    let build = Path::new("/work/build");
    let executable = Path::new("/work/poky/bitbake/bin/bitbake-layers");
    let authority = authority(build, executable, 11, &all_records());
    let planner = BitBakeLayersCommandPlanner::new(&authority, 11, build, executable).unwrap();
    let cases = vec![
        (BitBakeLayersOperation::ShowLayers, vec!["show-layers"]),
        (
            BitBakeLayersOperation::ShowRecipes {
                patterns: vec!["linux-*".into()],
                filenames: true,
                recipes_only: false,
                multiple: false,
                inherits: Some("kernel".into()),
                layer: None,
                bare: false,
                show_variants: true,
                multiconfig: None,
            },
            vec![
                "show-recipes",
                "-f",
                "--show-variants",
                "-i",
                "kernel",
                "linux-*",
            ],
        ),
        (
            BitBakeLayersOperation::ShowOverlayed {
                filenames: true,
                same_version: true,
                multiconfig: Some("mc-a".into()),
            },
            vec!["show-overlayed", "-f", "-s", "--mc", "mc-a"],
        ),
        (
            BitBakeLayersOperation::ShowAppends {
                patterns: vec!["linux-*".into()],
                multiconfig: None,
            },
            vec!["show-appends", "linux-*"],
        ),
        (
            BitBakeLayersOperation::ShowCrossDepends {
                filenames: true,
                ignored_layers: Some("core".into()),
            },
            vec!["show-cross-depends", "-f", "-i", "core"],
        ),
        (
            BitBakeLayersOperation::CreateLayer {
                directory: "/layers/meta-demo".into(),
                add: false,
                layer_id: None,
                priority: None,
                example_recipe: None,
                example_version: None,
            },
            vec!["create-layer", "/layers/meta-demo"],
        ),
        (
            BitBakeLayersOperation::CreateLayer {
                directory: "/layers/meta-demo".into(),
                add: true,
                layer_id: Some("demo".into()),
                priority: Some("7".into()),
                example_recipe: Some("example".into()),
                example_version: Some("1.0".into()),
            },
            vec![
                "create-layer",
                "--add-layer",
                "--layerid",
                "demo",
                "--priority",
                "7",
                "--example-recipe-name",
                "example",
                "--example-recipe-version",
                "1.0",
                "/layers/meta-demo",
            ],
        ),
        (
            BitBakeLayersOperation::AddLayers {
                directories: vec!["/layers/meta-one".into(), "/layers/meta-two".into()],
            },
            vec!["add-layer", "/layers/meta-one", "/layers/meta-two"],
        ),
        (
            BitBakeLayersOperation::RemoveLayers {
                directories: vec!["/layers/meta-old*".into()],
            },
            vec!["remove-layer", "/layers/meta-old*"],
        ),
        (
            BitBakeLayersOperation::Flatten {
                layers: vec!["meta-core".into()],
                output_directory: "/work/flattened".into(),
            },
            vec!["flatten", "meta-core", "/work/flattened"],
        ),
        (
            BitBakeLayersOperation::LayerIndexFetch {
                layers: vec!["meta-clang".into()],
                show_only: true,
                branch: Some("master".into()),
                shallow: true,
                ignored_layers: Some("core".into()),
                fetch_directory: Some("/work/layers".into()),
            },
            vec![
                "layerindex-fetch",
                "-n",
                "-s",
                "-b",
                "master",
                "-i",
                "core",
                "-f",
                "/work/layers",
                "meta-clang",
            ],
        ),
        (
            BitBakeLayersOperation::LayerIndexShowDepends {
                layers: vec!["meta-clang".into()],
                branch: Some("master".into()),
            },
            vec!["layerindex-show-depends", "-b", "master", "meta-clang"],
        ),
        (
            BitBakeLayersOperation::ShowMachines {
                bare: true,
                layer: Some("meta-aspeed".into()),
            },
            vec!["show-machines", "-b", "-l", "meta-aspeed"],
        ),
        (
            BitBakeLayersOperation::SaveBuildConf {
                layer_path: "/layers/meta-local".into(),
                template_name: "production".into(),
            },
            vec!["save-build-conf", "/layers/meta-local", "production"],
        ),
        (
            BitBakeLayersOperation::CreateLayersSetup {
                destination: "/work/setup".into(),
                output_prefix: Some("romulus".into()),
                writer: Some("oe-setup-layers".into()),
                json_only: true,
                update: true,
                custom_references: vec!["openbmc:master".into()],
            },
            vec![
                "create-layers-setup",
                "--json-only",
                "--update",
                "--output-prefix",
                "romulus",
                "--writer",
                "oe-setup-layers",
                "--use-custom-reference",
                "openbmc:master",
                "/work/setup",
            ],
        ),
    ];
    for (operation, expected) in cases {
        let command = planner.operation(&operation).unwrap();
        assert_eq!(command.executable(), executable);
        assert_eq!(command.arguments(), expected);
        assert_eq!(command.generation(), 11);
    }
}
