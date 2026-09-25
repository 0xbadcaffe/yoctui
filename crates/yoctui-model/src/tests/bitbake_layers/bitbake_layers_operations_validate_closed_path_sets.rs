use super::*;

#[test]
fn bitbake_layers_operations_validate_closed_path_sets() {
    BitBakeLayersOperation::CreateLayer {
        directory: "/layers/meta-demo".into(),
        add: true,
    }
    .validate()
    .unwrap();
    BitBakeLayersOperation::ShowRecipes {
        pattern: Some("linux-*".into()),
    }
    .validate()
    .unwrap();
    assert!(
        BitBakeLayersOperation::ShowRecipes {
            pattern: Some("\n".into())
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
}
