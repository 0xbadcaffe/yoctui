use super::*;

#[test]
fn bitbake_layers_operations_validate_closed_path_sets() {
    BitBakeLayersOperation::CreateLayer {
        directory: "/layers/meta-demo".into(),
        add: true,
        layer_id: None,
        priority: Some("7".into()),
        example_recipe: None,
        example_version: None,
    }
    .validate()
    .unwrap();
    BitBakeLayersOperation::ShowRecipes {
        patterns: vec!["linux-*".into()],
        filenames: false,
        recipes_only: false,
        multiple: false,
        inherits: None,
        layer: None,
        bare: false,
        show_variants: false,
        multiconfig: None,
    }
    .validate()
    .unwrap();
    assert!(
        BitBakeLayersOperation::ShowRecipes {
            patterns: vec!["\n".into()],
            filenames: false,
            recipes_only: false,
            multiple: false,
            inherits: None,
            layer: None,
            bare: false,
            show_variants: false,
            multiconfig: None,
        }
        .validate()
        .is_err()
    );
    BitBakeLayersOperation::AddLayers {
        directories: vec!["/layers/meta-one".into(), "/layers/meta-two".into()],
    }
    .validate()
    .unwrap();
    assert!(
        BitBakeLayersOperation::RemoveLayers {
            directories: Vec::new()
        }
        .validate()
        .is_err()
    );
    assert!(
        BitBakeLayersOperation::CreateLayersSetup {
            destination: "/work/setup".into(),
            output_prefix: None,
            writer: None,
            json_only: false,
            update: true,
            custom_references: vec!["missing-separator".into()],
        }
        .validate()
        .is_err()
    );
}
