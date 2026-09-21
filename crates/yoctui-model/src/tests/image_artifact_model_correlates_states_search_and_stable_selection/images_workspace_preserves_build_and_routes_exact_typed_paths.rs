use super::*;

#[test]
fn images_workspace_preserves_build_and_routes_exact_typed_paths() {
    let mut app = App::new(20, 20_000);
    app.workspace.recipes.push(Recipe {
        name: "core-image-minimal".into(),
        ..Recipe::default()
    });
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let request = ImageArtifactRequest {
        generation: 1,
        machine: "qemux86-64".into(),
    };
    assert_eq!(
        update(&mut app, Action::Open(Screen::Images)),
        Some(Effect::GetImageArtifacts(request.clone()))
    );
    let artifact_path = PathBuf::from("/build/tmp/deploy/images/qemux86-64/core-image-minimal.wic");
    let manifest_path =
        PathBuf::from("/build/tmp/deploy/images/qemux86-64/core-image-minimal.manifest");
    let artifact = ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: artifact_path.clone(),
        },
        kind: ImageArtifactKind::Wic,
        size_bytes: ImageArtifactField::Available(42),
        modified_unix_seconds: ImageArtifactField::Available(10),
        checksums: ImageArtifactField::Unavailable,
        manifests: ImageArtifactField::Available(vec![manifest_path.clone()]),
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Available(vec![artifact_path.clone()]),
    };
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryLoaded {
            request,
            inventory: ImageArtifactInventory {
                machine: "qemux86-64".into(),
                deploy_directory: ImageArtifactField::Available(
                    "/build/tmp/deploy/images/qemux86-64".into(),
                ),
                artifacts: vec![artifact],
            },
        },
    );
    assert_eq!(
        update(&mut app, Action::OpenSelectedImageArtifact),
        Some(Effect::OpenInEditor(artifact_path.clone()))
    );
    assert_eq!(
        update(
            &mut app,
            Action::OpenSelectedImageArtifactAssociation(ImageArtifactAssociation::Manifest)
        ),
        Some(Effect::OpenInEditor(manifest_path))
    );
    let _ = update(&mut app, Action::BeginSelectedImageArtifactBuild);
    assert_eq!(app.build.target.as_deref(), Some("core-image-minimal"));
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(BuildRequest { targets, .. }))
            if targets == &vec!["core-image-minimal".to_owned()]
    ));
}
