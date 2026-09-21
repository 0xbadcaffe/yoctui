use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::symlink;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "yoctui-sdk-tool-{name}-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
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

fn executable(path: &Path, body: &str) {
    crate::test_support::write_executable(path, body);
}

fn fixture(name: &str) -> (TestDirectory, SdkToolAdapter) {
    let directory = TestDirectory::new(name);
    let workspace = directory.path().join("workspace");
    let scripts = workspace.join("scripts");
    let build = directory.path().join("build");
    let deploy = directory.path().join("deploy/sdk");
    fs::create_dir_all(&scripts).unwrap();
    fs::create_dir_all(&build).unwrap();
    fs::create_dir_all(&deploy).unwrap();
    for tool in SDK_TOOL_NAMES {
        executable(&scripts.join(tool), "#!/bin/sh\nprintf '%s\\n' \"$@\"\n");
    }
    let adapter = SdkToolAdapter::new(build, deploy, vec![workspace]);
    (directory, adapter)
}

fn artifact(path: &Path) -> SdkArtifactIdentity {
    let metadata = fs::metadata(path).unwrap();
    SdkArtifactIdentity {
        path: path.into(),
        size_bytes: metadata.len(),
        modified_unix_seconds: modified_seconds(&metadata).unwrap(),
    }
}

fn publish_preview(adapter: &SdkToolAdapter, directory: &TestDirectory) -> SdkPublishPreview {
    let installer = adapter.sdk_deploy_root.join("poky-toolchain.sh");
    fs::write(&installer, b"installer").unwrap();
    let destination = directory.path().join("published");
    fs::create_dir(&destination).unwrap();
    let executable = match adapter.capability() {
        SdkToolCapability::Available {
            publish: Some(path),
            ..
        } => path,
        capability => panic!("unexpected capability: {capability:?}"),
    };
    SdkPublishPreview::new(executable, artifact(&installer), destination).unwrap()
}

fn native_preview(
    adapter: &SdkToolAdapter,
    mode: SdkNativeMode,
    extracted_root: Option<PathBuf>,
) -> SdkNativePreview {
    let capability = adapter.capability();
    let executable = capability.executable_for(mode).unwrap();
    SdkNativePreview::new(SdkNativeRequest {
        executable,
        mode,
        extracted_root,
        recipe: "cmake-native".into(),
        tool: (mode == SdkNativeMode::RunNative).then(|| "cmake".into()),
        arguments: if mode == SdkNativeMode::RunNative {
            vec!["--version".into()]
        } else {
            Vec::new()
        },
    })
    .unwrap()
}

#[cfg(unix)]
mod sdk_tool_capability_is_partial_and_rejects_unsafe_candidates;

mod sdk_tool_commands_reconstruct_exact_publication_and_native_argv;

#[cfg(unix)]
mod sdk_tool_commands_reject_unsafe_paths_and_stale_installer;

mod sdk_tool_extracted_environment_is_validated_and_child_only;

#[cfg(unix)]
mod sdk_tool_runner_confines_extracted_environment_to_the_child;

#[cfg(unix)]
mod sdk_tool_spawn_retries_only_transient_text_file_busy;

#[cfg(unix)]
mod sdk_tool_runner_streams_bounded_output_and_terminal_status;

#[cfg(unix)]
mod sdk_tool_runner_rejects_duplicate_and_reports_nonzero_and_loss;

#[cfg(unix)]
mod sdk_tool_runner_times_out_and_cancels_gracefully_or_forcibly;
