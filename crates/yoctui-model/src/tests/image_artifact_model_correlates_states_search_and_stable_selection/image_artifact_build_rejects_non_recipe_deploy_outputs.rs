use super::*;

#[test]
fn image_artifact_build_rejects_non_recipe_deploy_outputs() {
    let mut app = App::new(20, 20_000);
    app.workspace.recipes.push(Recipe {
        name: "core-image-minimal".into(),
        ..Recipe::default()
    });
    app.build.target = Some("core-image-minimal".into());
    let artifact = ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "bzImage--6.18.24-r0".into(),
            path: "/build/tmp/deploy/images/qemux86-64/bzImage--6.18.24-r0.bin".into(),
        },
        kind: ImageArtifactKind::Kernel,
        size_bytes: ImageArtifactField::Available(42),
        modified_unix_seconds: ImageArtifactField::Available(10),
        checksums: ImageArtifactField::Unavailable,
        manifests: ImageArtifactField::Unavailable,
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Unavailable,
    };
    app.image_artifact_selection = Some(artifact.identity.clone());
    app.image_artifacts = ImageArtifactInventoryState::Available {
        request: ImageArtifactRequest {
            generation: 1,
            machine: artifact.identity.machine.clone(),
        },
        inventory: ImageArtifactInventory {
            machine: artifact.identity.machine.clone(),
            deploy_directory: ImageArtifactField::Available(
                "/build/tmp/deploy/images/qemux86-64".into(),
            ),
            artifacts: vec![artifact],
        },
    };

    assert_eq!(
        update(&mut app, Action::BeginSelectedImageArtifactBuild),
        None
    );
    assert_eq!(app.build.target.as_deref(), Some("core-image-minimal"));
    assert!(app.active_dialog().is_none());
    assert!(app.notification.as_deref().is_some_and(|message| {
        message.contains("deployed artifact, not a buildable image recipe")
            && message.contains("Select an image recipe with i")
    }));
}
