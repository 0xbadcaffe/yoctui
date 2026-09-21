#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MaintenanceSstateAdapterError {
    #[error("invalid sstate input: {0}")]
    InvalidInput(String),
    #[error("unsafe sstate path: {0}")]
    UnsafePath(PathBuf),
    #[error("unsafe sstate executable: {0}")]
    UnsafeExecutable(PathBuf),
    #[error("stale sstate identity: {0}")]
    StaleIdentity(PathBuf),
    #[error("sstate capability is unavailable: {0}")]
    Unavailable(String),
    #[error("sstate preview does not match the typed request")]
    PreviewMismatch,
    #[error("sstate cleanup candidates changed after confirmation")]
    CandidateMismatch,
    #[error("sstate runner is already active")]
    Busy,
    #[error("sstate runner is not active")]
    NotRunning,
    #[error("failed to spawn sstate process: {0}")]
    Spawn(String),
    #[error("sstate {0:?} stream is unavailable")]
    StreamUnavailable(MaintenanceOutputStream),
    #[error("failed to control sstate process: {0}")]
    ProcessControl(String),
    #[error("sstate preview output is invalid: {0}")]
    InvalidPreviewOutput(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceSstateCapabilityInput {
    pub build_dir: PathBuf,
    pub sstate_dir: Option<PathBuf>,
    pub tmp_dir: Option<PathBuf>,
    pub stamps_dirs: Vec<PathBuf>,
    pub executable_search_path: Vec<PathBuf>,
}

pub struct MaintenanceSstateCapabilityInspector;

impl MaintenanceSstateCapabilityInspector {
    pub fn inspect(
        input: MaintenanceSstateCapabilityInput,
    ) -> Result<MaintenanceCapabilitySnapshot, MaintenanceSstateAdapterError> {
        let build_dir = canonical_directory(&input.build_dir)?;
        let mut limitations = Vec::new();
        let sstate_dir = input
            .sstate_dir
            .as_deref()
            .map(canonical_directory)
            .transpose()
            .map_err(|_| {
                MaintenanceSstateAdapterError::UnsafePath(
                    input.sstate_dir.clone().unwrap_or_default(),
                )
            })?;
        let tmp_dir = input
            .tmp_dir
            .as_deref()
            .map(canonical_directory)
            .transpose()
            .map_err(|_| {
                MaintenanceSstateAdapterError::UnsafePath(input.tmp_dir.clone().unwrap_or_default())
            })?;
        let mut stamps_dirs = input
            .stamps_dirs
            .iter()
            .map(|path| {
                canonical_directory(path)
                    .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        stamps_dirs.sort();
        stamps_dirs.dedup();
        stamps_dirs.truncate(MAX_MAINTENANCE_PATHS);

        let readiness = discover_tool(
            MaintenanceTool::OeCheckSstate,
            &["oe-check-sstate"],
            &input.executable_search_path,
            MaintenanceToolInterface::Native,
            &mut limitations,
        );
        let cleanup = if sstate_dir.is_some() {
            discover_cleanup_tool(&input.executable_search_path, &mut limitations)
        } else {
            MaintenanceToolCapability::Unavailable {
                tool: MaintenanceTool::SstateCacheManagement,
                reason: "SSTATE_DIR is unavailable".into(),
            }
        };
        let metadata = MaintenanceMetadata::new(MaintenanceMetadata {
            build_dir: Some(build_dir),
            sstate_dir,
            tmp_dir,
            stamps_dirs,
            ..MaintenanceMetadata::default()
        })
        .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))?;
        MaintenanceCapabilitySnapshot::new(metadata, vec![readiness, cleanup], limitations)
            .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))
    }
}

fn discover_cleanup_tool(
    search_path: &[PathBuf],
    limitations: &mut Vec<String>,
) -> MaintenanceToolCapability {
    for (name, interface) in [
        (
            "sstate-cache-management.py",
            MaintenanceToolInterface::SstatePython,
        ),
        (
            "sstate-cache-management.sh",
            MaintenanceToolInterface::SstateLegacyShell,
        ),
    ] {
        if let Some(identity) = find_executable(search_path, name, limitations) {
            return MaintenanceToolCapability::Available {
                tool: MaintenanceTool::SstateCacheManagement,
                executable: identity,
                interface,
            };
        }
    }
    MaintenanceToolCapability::Unavailable {
        tool: MaintenanceTool::SstateCacheManagement,
        reason: "neither sstate-cache-management.py nor legacy .sh is available".into(),
    }
}

fn discover_tool(
    tool: MaintenanceTool,
    names: &[&str],
    search_path: &[PathBuf],
    interface: MaintenanceToolInterface,
    limitations: &mut Vec<String>,
) -> MaintenanceToolCapability {
    for name in names {
        if let Some(identity) = find_executable(search_path, name, limitations) {
            return MaintenanceToolCapability::Available {
                tool,
                executable: identity,
                interface,
            };
        }
    }
    MaintenanceToolCapability::Unavailable {
        tool,
        reason: format!(
            "{} is not available in the configured tool search path",
            names[0]
        ),
    }
}

fn find_executable(
    search_path: &[PathBuf],
    name: &str,
    limitations: &mut Vec<String>,
) -> Option<MaintenanceFileIdentity> {
    for directory in search_path.iter().take(MAX_MAINTENANCE_PATHS) {
        let Ok(directory) = canonical_directory(directory) else {
            push_limitation(
                limitations,
                format!("ignored unsafe tool directory {}", directory.display()),
            );
            continue;
        };
        let candidate = directory.join(name);
        match executable_identity(&candidate, name) {
            Ok(identity) => return Some(identity),
            Err(MaintenanceSstateAdapterError::UnsafeExecutable(path)) if path.exists() => {
                push_limitation(
                    limitations,
                    format!("ignored unsafe executable {}", path.display()),
                );
            }
            _ => {}
        }
    }
    None
}

fn push_limitation(limitations: &mut Vec<String>, limitation: String) {
    yoctui_utils::push_unique_bounded(limitations, limitation, MAX_MAINTENANCE_LIMITATIONS);
}

fn safe_metadata(path: &Path, allow_directory: bool) -> Result<fs::Metadata, ()> {
    if !path.is_absolute() || path == Path::new("/") {
        return Err(());
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    if metadata.file_type().is_symlink()
        || (!metadata.is_file() && !(allow_directory && metadata.is_dir()))
    {
        return Err(());
    }
    Ok(metadata)
}

fn canonical_directory(path: &Path) -> Result<PathBuf, MaintenanceSstateAdapterError> {
    let metadata = safe_metadata(path, true)
        .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
    if !metadata.is_dir() {
        return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
    if canonical != path {
        return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
    }
    Ok(canonical)
}

fn executable_identity(
    path: &Path,
    expected_name: &str,
) -> Result<MaintenanceFileIdentity, MaintenanceSstateAdapterError> {
    if path.file_name() != Some(OsStr::new(expected_name)) {
        return Err(MaintenanceSstateAdapterError::UnsafeExecutable(path.into()));
    }
    let metadata = safe_metadata(path, false)
        .map_err(|_| MaintenanceSstateAdapterError::UnsafeExecutable(path.into()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(MaintenanceSstateAdapterError::UnsafeExecutable(path.into()));
        }
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceSstateAdapterError::UnsafeExecutable(path.into()))?;
    if canonical != path {
        return Err(MaintenanceSstateAdapterError::UnsafeExecutable(path.into()));
    }
    MaintenanceFileIdentity::new(
        canonical,
        metadata.len(),
        metadata
            .modified()
            .map_err(|_| MaintenanceSstateAdapterError::UnsafeExecutable(path.into()))?,
    )
    .map_err(|_| MaintenanceSstateAdapterError::UnsafeExecutable(path.into()))
}

fn revalidate_executable(
    identity: &MaintenanceFileIdentity,
    expected_name: &str,
) -> Result<(), MaintenanceSstateAdapterError> {
    let current = executable_identity(&identity.path, expected_name)?;
    if &current != identity {
        return Err(MaintenanceSstateAdapterError::StaleIdentity(
            identity.path.clone(),
        ));
    }
    Ok(())
}

fn validate_output_path(path: &Path) -> Result<(), MaintenanceSstateAdapterError> {
    if !path.is_absolute() || path == Path::new("/") {
        return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
    }
    if path.exists() {
        let metadata = safe_metadata(path, false)
            .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
        if !metadata.is_file() {
            return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
        }
        let canonical = fs::canonicalize(path)
            .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
        if canonical != path {
            return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
        }
    } else {
        let parent = path
            .parent()
            .ok_or_else(|| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
        canonical_directory(parent)?;
    }
    Ok(())
}
