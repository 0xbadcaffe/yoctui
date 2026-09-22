use super::*;

fn artifact(image: &str, file: &str, kind: ImageArtifactKind) -> ImageArtifact {
    ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "romulus".into(),
            image: image.into(),
            path: format!("/build/tmp/deploy/images/romulus/{file}").into(),
        },
        kind,
        size_bytes: ImageArtifactField::Available(42),
        modified_unix_seconds: ImageArtifactField::Available(10),
        checksums: ImageArtifactField::Unavailable,
        manifests: ImageArtifactField::Unavailable,
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Unavailable,
    }
}

#[test]
fn rootfs_tabs_resolve_the_built_image_instead_of_an_unowned_deploy_file() {
    let mut app = App::new(20, 20_000);
    app.workspace.recipes = vec![Recipe {
        name: "obmc-phosphor-image".into(),
        ..Recipe::default()
    }];
    app.build.target = Some("obmc-phosphor-image".into());
    let fit = artifact("fit-image", "fit-image.its", ImageArtifactKind::Other);
    let image = artifact(
        "obmc-phosphor-image",
        "obmc-phosphor-image-romulus.jffs2",
        ImageArtifactKind::RootFilesystem,
    );
    app.image_artifacts = ImageArtifactInventoryState::Available {
        request: ImageArtifactRequest {
            generation: 1,
            machine: "romulus".into(),
        },
        inventory: ImageArtifactInventory {
            machine: "romulus".into(),
            deploy_directory: ImageArtifactField::Available(
                "/build/tmp/deploy/images/romulus".into(),
            ),
            artifacts: vec![fit.clone(), image.clone()],
        },
    };
    app.image_artifact_selection = Some(fit.identity);

    let effect = update(&mut app, Action::ShiftImagesView { delta: 1 });
    assert_eq!(app.images_view, ImagesView::RootfsPackages);
    assert_eq!(app.image_artifact_selection, Some(image.identity.clone()));
    assert_eq!(
        effect,
        Some(Effect::GetRootfsComposition(RootfsCompositionRequest {
            generation: 1,
            image: image.identity,
        }))
    );
}
