use super::*;

#[cfg(unix)]
pub(crate) fn qemu_workspace_fixture(name: &str, body: &str) -> (PathBuf, PathBuf, App) {
    use std::os::unix::fs::PermissionsExt;

    let directory = std::env::temp_dir().join(format!(
        "yoctui-qemu-workspace-{}-{name}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let build_dir = directory.join("build");
    let deploy = directory.join("qemux86-64");
    fs::create_dir_all(&build_dir).unwrap();
    fs::create_dir_all(&deploy).unwrap();
    let executable = directory.join("runqemu");
    fs::write(&executable, format!("#!/bin/sh\n{body}\n")).unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&executable, permissions).unwrap();
    let image_path = deploy.join("core-image-minimal.wic");
    fs::write(&image_path, b"wic").unwrap();
    let identity = yoctui_model::ImageArtifactIdentity {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        path: image_path,
    };
    let artifact = yoctui_model::ImageArtifact {
        identity: identity.clone(),
        kind: yoctui_model::ImageArtifactKind::Wic,
        size_bytes: yoctui_model::ImageArtifactField::Available(3),
        modified_unix_seconds: yoctui_model::ImageArtifactField::Unavailable,
        checksums: yoctui_model::ImageArtifactField::Unavailable,
        manifests: yoctui_model::ImageArtifactField::Unavailable,
        licenses: yoctui_model::ImageArtifactField::Unavailable,
        spdx: yoctui_model::ImageArtifactField::Unavailable,
        wic_files: yoctui_model::ImageArtifactField::Unavailable,
    };
    let mut app = App::new(20, 20_000);
    app.screen = Screen::Images;
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.image_artifact_selection = Some(identity);
    app.image_artifacts = yoctui_model::ImageArtifactInventoryState::Available {
        request: ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        },
        inventory: yoctui_model::ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: yoctui_model::ImageArtifactField::Available(deploy),
            artifacts: vec![artifact],
        },
    };
    execute_qemu_capability_effect(
        &mut app,
        &QemuCapabilityInspector::with_executable(executable),
        Effect::InspectQemuCapability,
    );
    (directory, build_dir, app)
}
