use super::*;
use std::{
    fs::File,
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "yoctui-image-artifacts-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> TestDirectory {
    TestDirectory::new()
}

fn request() -> ImageArtifactRequest {
    ImageArtifactRequest {
        generation: 1,
        machine: "qemux86-64".into(),
    }
}

mod image_artifact_adapter_classifies_only_uncompressed_wic_images;

mod boot_artifact_identity_never_becomes_a_bitbake_recipe_target;

mod image_artifact_adapter_scans_and_classifies_deterministically;

mod image_artifact_adapter_reports_empty_partial_malformed_and_oversized_inputs;

#[cfg(unix)]
mod image_artifact_adapter_rejects_symlink_missing_escape_and_machine_mismatch;

mod image_artifact_adapter_supports_timeout_and_cancellation;
