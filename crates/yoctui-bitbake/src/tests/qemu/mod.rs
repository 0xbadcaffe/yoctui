use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use yoctui_model::{ImageArtifactField, QemuLaunchDraft};

static FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

fn fixture_dir(name: &str) -> PathBuf {
    let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "yoctui-qemu-adapter-{}-{name}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).unwrap();
    fs::canonicalize(directory).unwrap()
}

#[cfg(unix)]
fn executable(path: &Path, body: &str) {
    crate::test_support::write_executable(path, &format!("#!/bin/sh\n{body}\n"));
}

fn artifact(path: PathBuf, kind: ImageArtifactKind) -> ImageArtifact {
    ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path,
        },
        kind,
        size_bytes: ImageArtifactField::Unavailable,
        modified_unix_seconds: ImageArtifactField::Unavailable,
        checksums: ImageArtifactField::Unavailable,
        manifests: ImageArtifactField::Unavailable,
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Unavailable,
    }
}

#[cfg(unix)]
fn fixture_preview(name: &str, body: &str) -> (PathBuf, QemuLaunchPreview, QemuCommandSpec) {
    let directory = fixture_dir(name);
    let program = directory.join("runqemu");
    executable(&program, body);
    let deploy = directory.join("qemux86-64");
    fs::create_dir(&deploy).unwrap();
    let image_path = deploy.join("core-image-minimal.wic");
    fs::write(&image_path, b"wic").unwrap();
    let image = artifact(image_path, ImageArtifactKind::Wic);
    let capability =
        QemuCapabilityInspector::with_executable(program).inspect(std::slice::from_ref(&image));
    let draft = QemuLaunchDraft::for_artifact(image.identity, image.kind);
    let preview = draft.preview(&capability).unwrap();
    let command = QemuCommandSpec::from_preview(&preview).unwrap();
    (directory, preview, command)
}

#[cfg(unix)]
mod qemu_adapter_capability_distinguishes_available_missing_image_and_failure;

#[cfg(unix)]
mod qemu_adapter_builds_exact_shell_free_arguments_and_rejects_tampering;

#[cfg(unix)]
mod qemu_adapter_rejects_symlinked_artifacts;

#[cfg(unix)]
mod qemu_adapter_streams_bounded_output_and_reports_nonzero_exit;

#[cfg(unix)]
mod qemu_adapter_reports_successful_completion;

#[cfg(unix)]
mod qemu_adapter_cancels_gracefully_and_escalates_process_groups;

#[cfg(unix)]
mod qemu_adapter_reports_unexpected_output_channel_loss;
