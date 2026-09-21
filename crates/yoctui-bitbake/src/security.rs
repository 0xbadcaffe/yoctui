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
#[path = "tests/security/mod.rs"]
mod tests;
