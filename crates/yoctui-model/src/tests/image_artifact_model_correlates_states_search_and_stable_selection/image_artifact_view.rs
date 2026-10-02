use super::*;

fn fixture() -> (App, PathBuf, PathBuf) {
    let mut app = App::new(20, 20_000);
    app.screen = Screen::Images;
    let root = PathBuf::from("/deploy/qemux86-64");
    let path = root.join("board.dtb");
    let artifact = ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: path.clone(),
        },
        kind: ImageArtifactKind::Kernel,
        size_bytes: ImageArtifactField::Available(42),
        modified_unix_seconds: ImageArtifactField::Available(0),
        checksums: ImageArtifactField::Unavailable,
        manifests: ImageArtifactField::Available(vec![root.join("image.manifest")]),
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Unavailable,
    };
    app.image_artifact_selection = Some(artifact.identity.clone());
    app.image_artifacts = ImageArtifactInventoryState::Available {
        request: ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        },
        inventory: ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: ImageArtifactField::Available(root.clone()),
            artifacts: vec![artifact],
        },
    };
    (app, root, path)
}

#[test]
fn image_artifact_view_opens_text_in_normal_mode_and_device_tree_in_review_only() {
    let (mut app, root, path) = fixture();
    let effect = update(&mut app, Action::OpenSelectedImageArtifact);
    assert_eq!(
        effect,
        Some(Effect::ViewImageArtifact {
            root: root.clone(),
            path: path.clone()
        })
    );
    assert!(app.active_dialog().is_none());
    let result = ImageArtifactView::DeviceTree {
        kind: PlatformFileKind::Dtb,
        program: "/usr/bin/dtc".into(),
        size_bytes: 42,
    };
    assert_eq!(
        update(
            &mut app,
            Action::ImageArtifactViewed {
                root: root.clone(),
                path: path.clone(),
                result: Ok(result)
            }
        ),
        None
    );
    let Some(Dialog::DtcDecompile(dialog)) = app.active_dialog() else {
        panic!("expected review form")
    };
    assert_eq!(dialog.source, path);
    assert_eq!(dialog.component, PlatformComponent::Images);
    assert!(dialog.view_after);
    assert_eq!(
        dialog.terminal_request().arguments,
        vec![
            "-I",
            "dtb",
            "-O",
            "dts",
            "-o",
            "/deploy/qemux86-64/board.yoctui.dts",
            "/deploy/qemux86-64/board.dtb"
        ]
    );
    update(&mut app, Action::DtcDecompile(DtcDecompileAction::Cancel));
    let path = root.join("image.manifest");
    update(
        &mut app,
        Action::ImageArtifactViewed {
            root: root.clone(),
            path: path.clone(),
            result: Ok(ImageArtifactView::Text("package 1.0\n".into())),
        },
    );
    let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
        panic!("expected internal viewer")
    };
    assert_eq!(editor.selected_path(), Some(path));
    assert_eq!(editor.document.text, "package 1.0\n");
    assert_eq!(editor.document.mode(), TextAreaMode::Normal);
    assert_eq!(editor.focus, RecipeEditorFocus::Document);
    assert!(!editor.is_dirty());
}

#[test]
fn image_artifact_view_rejects_stale_outside_unknown_and_covered_results() {
    for (root, path) in [
        ("/other", "/deploy/qemux86-64/board.dtb"),
        ("/deploy/qemux86-64", "/outside/board.dtb"),
        ("/deploy/qemux86-64", "/deploy/qemux86-64/unknown.dts"),
    ] {
        let (mut app, _, _) = fixture();
        update(
            &mut app,
            Action::ImageArtifactViewed {
                root: root.into(),
                path: path.into(),
                result: Ok(ImageArtifactView::Text("stale".into())),
            },
        );
        assert!(app.active_dialog().is_none());
    }
    let (mut app, root, path) = fixture();
    app.dialogs.push_back(Dialog::BuildOptions);
    let before = app.active_dialog().cloned();
    update(
        &mut app,
        Action::ImageArtifactViewed {
            root,
            path,
            result: Ok(ImageArtifactView::Text("covered".into())),
        },
    );
    assert_eq!(app.active_dialog(), before.as_ref());
}

#[test]
fn image_artifact_view_failure_is_explicit_and_missing_authority_does_not_open() {
    let (mut app, root, path) = fixture();
    update(
        &mut app,
        Action::ImageArtifactViewed {
            root,
            path,
            result: Err("dtc is unavailable".into()),
        },
    );
    assert_eq!(app.notification.as_deref(), Some("dtc is unavailable"));
    assert!(app.active_dialog().is_none());
    let ImageArtifactInventoryState::Available { inventory, .. } = &mut app.image_artifacts else {
        unreachable!()
    };
    inventory.deploy_directory = ImageArtifactField::Unavailable;
    assert_eq!(update(&mut app, Action::OpenSelectedImageArtifact), None);
    assert!(app.notification.as_ref().unwrap().contains("authoritative"));
}
