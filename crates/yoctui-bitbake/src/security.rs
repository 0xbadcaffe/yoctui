use std::{
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;
use yoctui_model::{
    MAX_SECURITY_PATHS, SecurityCapabilitySnapshot, SecurityMapperCapability, SecurityScope,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const MAX_SECURITY_PATH_DIRECTORIES: usize = 256;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SecurityCapabilityError {
    #[error("Security build directory is unsafe: {0}")]
    UnsafeBuildDirectory(PathBuf),
    #[error("Security scope is invalid")]
    InvalidScope,
    #[error("too many Security capability inputs")]
    TooManyInputs,
    #[error("Security capability snapshot is invalid: {0}")]
    InvalidSnapshot(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityCapabilityInput {
    pub release: Option<String>,
    pub build_directory: PathBuf,
    pub scope: SecurityScope,
    pub available_scopes: Vec<SecurityScope>,
    pub reported_tasks: Vec<String>,
    pub image_build_emits_sbom: bool,
    pub cve_roots: Vec<PathBuf>,
    pub sbom_roots: Vec<PathBuf>,
    pub path_directories: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SecurityCapabilityInspector {
    input: SecurityCapabilityInput,
}

impl SecurityCapabilityInspector {
    pub fn new(input: SecurityCapabilityInput) -> Self {
        Self { input }
    }

    pub fn inspect(&self) -> Result<SecurityCapabilitySnapshot, SecurityCapabilityError> {
        if !self.input.scope.is_valid()
            || !scope_identity_is_safe(&self.input.scope)
            || self
                .input
                .available_scopes
                .iter()
                .any(|scope| !scope.is_valid() || !scope_identity_is_safe(scope))
        {
            return Err(SecurityCapabilityError::InvalidScope);
        }
        if self.input.available_scopes.len() > MAX_SECURITY_PATHS
            || self.input.reported_tasks.len() > MAX_SECURITY_PATHS
            || self.input.cve_roots.len() > MAX_SECURITY_PATHS
            || self.input.sbom_roots.len() > MAX_SECURITY_PATHS
            || self.input.path_directories.len() > MAX_SECURITY_PATH_DIRECTORIES
        {
            return Err(SecurityCapabilityError::TooManyInputs);
        }
        let build_directory =
            canonical_directory(&self.input.build_directory).ok_or_else(|| {
                SecurityCapabilityError::UnsafeBuildDirectory(self.input.build_directory.clone())
            })?;
        let mut limitations = Vec::new();
        let cve_roots = canonical_optional_directories(
            &self.input.cve_roots,
            "CVE report root",
            &mut limitations,
        );
        let sbom_roots = canonical_optional_directories(
            &self.input.sbom_roots,
            "SPDX report root",
            &mut limitations,
        );
        let path_directories = canonical_optional_directories(
            &self.input.path_directories,
            "Security PATH directory",
            &mut limitations,
        );
        let mapper_path =
            discover_executable(&path_directories, "cve-check-map-pkgs", &mut limitations);
        let mapper = mapper_path.and_then(|executable| {
            cve_roots.first().map_or_else(
                || {
                    limitations.push(
                        "cve-check-map-pkgs is available, but no canonical CVE report root is available"
                            .into(),
                    );
                    None
                },
                |root| {
                    Some(SecurityMapperCapability {
                        executable,
                        arguments: vec![root.display().to_string()],
                    })
                },
            )
        });
        let reported = self
            .input
            .reported_tasks
            .iter()
            .map(|task| task.strip_prefix("do_").unwrap_or(task))
            .collect::<Vec<_>>();
        let cve_task = reported
            .contains(&"cve_check")
            .then_some("cve_check".into());
        let recipe_sbom_task = ["create_recipe_sbom", "create_spdx"]
            .into_iter()
            .find(|candidate| reported.contains(candidate))
            .map(str::to_owned);
        let image_sbom_task = ["create_rootfs_sbom", "create_image_sbom", "create_spdx"]
            .into_iter()
            .find(|candidate| reported.contains(candidate))
            .map(str::to_owned);
        SecurityCapabilitySnapshot::new(
            self.input.release.clone(),
            build_directory,
            self.input.scope.clone(),
            self.input.available_scopes.clone(),
            cve_task,
            recipe_sbom_task,
            image_sbom_task,
            self.input.image_build_emits_sbom,
            mapper,
            cve_roots,
            sbom_roots,
            limitations,
        )
        .map_err(|message| SecurityCapabilityError::InvalidSnapshot(message.into()))
    }
}

fn canonical_optional_directories(
    paths: &[PathBuf],
    label: &str,
    limitations: &mut Vec<String>,
) -> Vec<PathBuf> {
    let mut accepted = Vec::new();
    for path in paths {
        match canonical_directory(path) {
            Some(path) => {
                if !accepted.contains(&path) {
                    accepted.push(path);
                }
            }
            None => limitations.push(format!("ignored unsafe {label}: {}", path.display())),
        }
    }
    accepted.sort();
    accepted
}

fn canonical_directory(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    let canonical = fs::canonicalize(path).ok()?;
    (canonical == path).then_some(canonical)
}

fn scope_identity_is_safe(scope: &SecurityScope) -> bool {
    match scope {
        SecurityScope::Recipe(identity) => {
            let Ok(metadata) = fs::symlink_metadata(&identity.file) else {
                return false;
            };
            !metadata.file_type().is_symlink()
                && metadata.is_file()
                && fs::canonicalize(&identity.file).ok().as_ref() == Some(&identity.file)
        }
        SecurityScope::Image { .. } => true,
    }
}

fn discover_executable(
    directories: &[PathBuf],
    name: &str,
    limitations: &mut Vec<String>,
) -> Option<PathBuf> {
    for directory in directories {
        let candidate = directory.join(name);
        let Ok(metadata) = fs::symlink_metadata(&candidate) else {
            continue;
        };
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || fs::canonicalize(&candidate).ok().as_ref() != Some(&candidate)
            || !is_executable(&metadata)
        {
            limitations.push(format!(
                "ignored unsafe Security executable candidate: {}",
                candidate.display()
            ));
            continue;
        }
        return Some(candidate);
    }
    None
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    true
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn security_capability_preserves_current_and_legacy_reported_tasks() {
        let (_directory, current) = fixture(&["do_cve_check", "do_create_recipe_sbom"]);
        let current = SecurityCapabilityInspector::new(current).inspect().unwrap();
        assert_eq!(current.cve_task.as_deref(), Some("cve_check"));
        assert_eq!(
            current.recipe_sbom_task.as_deref(),
            Some("create_recipe_sbom")
        );
        assert!(current.mapper.is_some());
        assert_eq!(current.cve_roots.len(), 1);

        let (_directory, legacy) = fixture(&["do_cve_check", "do_create_spdx"]);
        let legacy = SecurityCapabilityInspector::new(legacy).inspect().unwrap();
        assert_eq!(legacy.recipe_sbom_task.as_deref(), Some("create_spdx"));
        assert_eq!(legacy.image_sbom_task.as_deref(), Some("create_spdx"));
    }

    #[test]
    fn security_capability_is_partial_for_unsafe_optional_inputs() {
        let (directory, mut input) = fixture(&["do_cve_check"]);
        input.cve_roots.push(directory.path().join("missing"));
        input
            .path_directories
            .push(directory.path().join("missing-bin"));
        input.reported_tasks.push("do_unrelated".into());
        let snapshot = SecurityCapabilityInspector::new(input).inspect().unwrap();
        assert_eq!(snapshot.cve_task.as_deref(), Some("cve_check"));
        assert!(snapshot.recipe_sbom_task.is_none());
        assert_eq!(snapshot.limitations.len(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn security_capability_rejects_primary_symlinks_and_ignores_tool_symlinks() {
        use std::os::unix::fs::symlink;

        let (directory, mut input) = fixture(&["do_cve_check"]);
        let real_build = input.build_directory.clone();
        let linked_build = directory.path().join("linked-build");
        symlink(&real_build, &linked_build).unwrap();
        input.build_directory = linked_build;
        assert!(matches!(
            SecurityCapabilityInspector::new(input).inspect(),
            Err(SecurityCapabilityError::UnsafeBuildDirectory(_))
        ));

        let (_directory, input) = fixture(&["do_cve_check"]);
        let mapper = input.path_directories[0].join("cve-check-map-pkgs");
        fs::remove_file(&mapper).unwrap();
        symlink("/bin/sh", &mapper).unwrap();
        let snapshot = SecurityCapabilityInspector::new(input).inspect().unwrap();
        assert!(snapshot.mapper.is_none());
        assert!(
            snapshot
                .limitations
                .iter()
                .any(|value| value.contains("unsafe Security executable"))
        );
    }

    #[test]
    fn security_capability_bounds_inputs_and_fails_closed_for_invalid_scope() {
        let (_directory, mut input) = fixture(&[]);
        input.path_directories = (0..=MAX_SECURITY_PATH_DIRECTORIES)
            .map(|index| PathBuf::from(format!("/missing/{index}")))
            .collect();
        assert_eq!(
            SecurityCapabilityInspector::new(input).inspect(),
            Err(SecurityCapabilityError::TooManyInputs)
        );

        let (_directory, mut input) = fixture(&[]);
        input.scope = SecurityScope::Image {
            target: "../bad".into(),
            machine: "qemu".into(),
            distro: "poky".into(),
        };
        assert_eq!(
            SecurityCapabilityInspector::new(input).inspect(),
            Err(SecurityCapabilityError::InvalidScope)
        );
    }
}
