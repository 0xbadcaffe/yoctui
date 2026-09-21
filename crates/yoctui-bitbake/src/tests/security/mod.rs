use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
use yoctui_model::RecipeIdentity;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "yoctui-security-capability-{}-{}",
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

fn scope(provider: PathBuf) -> SecurityScope {
    SecurityScope::Recipe(RecipeIdentity {
        name: "busybox".into(),
        file: provider,
    })
}

fn executable(path: &Path) {
    crate::test_support::write_executable(path, "#!/bin/sh\n");
}

fn fixture(tasks: &[&str]) -> (TestDirectory, SecurityCapabilityInput) {
    let directory = TestDirectory::new();
    let build = directory.path().join("build");
    let layer = directory.path().join("layer");
    let cve = build.join("tmp/log/cve");
    let spdx = build.join("tmp/deploy/spdx");
    let bin = directory.path().join("bin");
    for path in [&build, &layer, &cve, &spdx, &bin] {
        fs::create_dir_all(path).unwrap();
    }
    let provider = layer.join("busybox.bb");
    fs::write(&provider, b"SUMMARY = \"busybox\"\n").unwrap();
    executable(&bin.join("cve-check-map-pkgs"));
    let scope = scope(provider);
    let input = SecurityCapabilityInput {
        release: Some("6.0".into()),
        build_directory: build,
        scope: scope.clone(),
        available_scopes: vec![scope],
        reported_tasks: tasks.iter().map(|task| (*task).into()).collect(),
        image_build_emits_sbom: false,
        cve_roots: vec![cve],
        sbom_roots: vec![spdx],
        path_directories: vec![bin],
    };
    (directory, input)
}

mod security_capability_preserves_current_and_legacy_reported_tasks;

mod security_capability_is_partial_for_unsafe_optional_inputs;

#[cfg(unix)]
mod security_capability_rejects_primary_symlinks_and_ignores_tool_symlinks;

mod security_capability_bounds_inputs_and_fails_closed_for_invalid_scope;
