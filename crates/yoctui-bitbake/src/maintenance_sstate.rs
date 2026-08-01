use std::{
    collections::{BTreeMap, VecDeque},
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::{Child, Command},
    time::Instant,
};
use yoctui_model::{
    MAX_MAINTENANCE_LIMITATIONS, MAX_MAINTENANCE_PATHS, MAX_MAINTENANCE_TEXT_BYTES,
    MaintenanceCapabilitySnapshot, MaintenanceFileIdentity, MaintenanceMetadata,
    MaintenanceOperation, MaintenanceOperationPreview, MaintenanceOutputStream,
    MaintenanceSessionId, MaintenanceTool, MaintenanceToolCapability, MaintenanceToolInterface,
    PrServiceOperation, PrServiceRequest, SstateCleanupMode, SstateCleanupPreview,
    SstateCleanupRequest, SstateReadinessMode, SstateReadinessRequest,
};

const SSTATE_EVENT_CHANNEL_CAPACITY: usize = 64;
const SSTATE_OPERATION_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const MAX_PREVIEW_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

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
    if limitations.len() < MAX_MAINTENANCE_LIMITATIONS && !limitations.contains(&limitation) {
        limitations.push(limitation);
    }
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

fn identity_for_candidate(
    path: &Path,
    cache_dir: &Path,
) -> Result<MaintenanceFileIdentity, MaintenanceSstateAdapterError> {
    if !path.starts_with(cache_dir) || path == cache_dir {
        return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
    }
    let metadata = safe_metadata(path, false)
        .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
    if canonical != path || !canonical.starts_with(cache_dir) {
        return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
    }
    MaintenanceFileIdentity::new(
        canonical,
        metadata.len(),
        metadata
            .modified()
            .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?,
    )
    .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))
}

fn available_tool(
    snapshot: &MaintenanceCapabilitySnapshot,
    tool: MaintenanceTool,
) -> Result<(&MaintenanceFileIdentity, MaintenanceToolInterface), MaintenanceSstateAdapterError> {
    match snapshot.capability(tool) {
        Some(MaintenanceToolCapability::Available {
            executable,
            interface,
            ..
        }) => Ok((executable, *interface)),
        Some(MaintenanceToolCapability::Unavailable { reason, .. }) => {
            Err(MaintenanceSstateAdapterError::Unavailable(reason.clone()))
        }
        None => Err(MaintenanceSstateAdapterError::Unavailable(
            "capability was not inspected".into(),
        )),
    }
}

fn expected_name(tool: MaintenanceToolInterface, path: &Path) -> Option<&str> {
    match tool {
        MaintenanceToolInterface::Native => path.file_name().and_then(OsStr::to_str),
        MaintenanceToolInterface::SstatePython => Some("sstate-cache-management.py"),
        MaintenanceToolInterface::SstateLegacyShell => Some("sstate-cache-management.sh"),
        MaintenanceToolInterface::DetectionOnly => None,
    }
}

fn readiness_arguments(request: &SstateReadinessRequest) -> Vec<String> {
    let mut arguments = Vec::new();
    if let Some(output) = &request.output {
        arguments.extend(["--outfile".into(), output.display().to_string()]);
    }
    if let Some(log) = &request.log {
        arguments.extend(["--log".into(), log.display().to_string()]);
    }
    if request.mode == SstateReadinessMode::SameTmpdir {
        arguments.push("--same-tmpdir".into());
    }
    arguments.extend(request.targets.iter().cloned());
    arguments
}

fn cleanup_arguments(request: &SstateCleanupRequest, preview: bool, execute: bool) -> Vec<String> {
    let mut arguments = vec![
        "--cache-dir".into(),
        request.cache_dir.display().to_string(),
    ];
    for mode in &request.modes {
        match mode {
            SstateCleanupMode::Duplicates => arguments.push("--remove-duplicated".into()),
            SstateCleanupMode::Orphans => arguments.push("--remove-orphans".into()),
            SstateCleanupMode::UnreferencedByStamps => {
                for stamps in &request.stamps_dirs {
                    arguments.push("--stamps-dir".into());
                    arguments.push(stamps.display().to_string());
                }
            }
        }
    }
    arguments.extend(["--jobs".into(), request.jobs.to_string()]);
    if preview {
        arguments.push("--debug".into());
    }
    if execute {
        arguments.push("--yes".into());
    }
    arguments
}

fn pr_service_arguments(request: &PrServiceRequest) -> Vec<String> {
    vec![
        match request.operation {
            PrServiceOperation::Export => "export",
            PrServiceOperation::Import => "import",
        }
        .into(),
        request.file.display().to_string(),
    ]
}

fn filesystem_identity(
    path: &Path,
    directory: bool,
) -> Result<FilesystemIdentity, MaintenanceSstateAdapterError> {
    let metadata = safe_metadata(path, directory)
        .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
    if metadata.is_dir() != directory {
        return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
    if canonical != path {
        return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
    }
    Ok(FilesystemIdentity {
        path: canonical,
        byte_size: metadata.len(),
        modified_at: metadata
            .modified()
            .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?,
        directory,
    })
}

pub(crate) fn guard_regular_file(
    path: &Path,
) -> Result<MaintenanceFilesystemGuard, MaintenanceSstateAdapterError> {
    Ok(MaintenanceFilesystemGuard::Existing(filesystem_identity(
        path, false,
    )?))
}

pub(crate) fn guard_directory(
    path: &Path,
) -> Result<MaintenanceFilesystemGuard, MaintenanceSstateAdapterError> {
    Ok(MaintenanceFilesystemGuard::Existing(filesystem_identity(
        path, true,
    )?))
}

pub(crate) fn guard_directory_or_absent(
    path: &Path,
) -> Result<MaintenanceFilesystemGuard, MaintenanceSstateAdapterError> {
    if path.exists() {
        return guard_directory(path);
    }
    let parent = path
        .parent()
        .ok_or_else(|| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
    Ok(MaintenanceFilesystemGuard::Absent {
        path: path.into(),
        parent: filesystem_identity(parent, true)?,
    })
}

fn revalidate_filesystem_guard(
    guard: &MaintenanceFilesystemGuard,
) -> Result<(), MaintenanceSstateAdapterError> {
    match guard {
        MaintenanceFilesystemGuard::Existing(expected) => {
            let current = filesystem_identity(&expected.path, expected.directory)?;
            if &current != expected {
                return Err(MaintenanceSstateAdapterError::StaleIdentity(
                    expected.path.clone(),
                ));
            }
        }
        MaintenanceFilesystemGuard::Absent { path, parent } => {
            if path.exists() || filesystem_identity(&parent.path, true)? != *parent {
                return Err(MaintenanceSstateAdapterError::StaleIdentity(path.clone()));
            }
        }
    }
    Ok(())
}

fn require_writable(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<(), MaintenanceSstateAdapterError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o222 == 0 {
            return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
        }
    }
    #[cfg(not(unix))]
    if metadata.permissions().readonly() {
        return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
    }
    Ok(())
}

fn inspect_pr_service_file(
    request: &PrServiceRequest,
) -> Result<PrServiceFileGuard, MaintenanceSstateAdapterError> {
    match request.operation {
        PrServiceOperation::Export => {
            let parent = request
                .file
                .parent()
                .ok_or_else(|| MaintenanceSstateAdapterError::UnsafePath(request.file.clone()))?;
            let parent_identity = filesystem_identity(parent, true)?;
            let parent_metadata = fs::metadata(parent)
                .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(parent.into()))?;
            require_writable(parent, &parent_metadata)?;
            let existing = if request.file.exists() {
                let identity = filesystem_identity(&request.file, false)?;
                let metadata = fs::metadata(&request.file)
                    .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(request.file.clone()))?;
                require_writable(&request.file, &metadata)?;
                Some(identity)
            } else {
                None
            };
            Ok(PrServiceFileGuard::Export {
                parent: parent_identity,
                existing,
            })
        }
        PrServiceOperation::Import => {
            let identity = filesystem_identity(&request.file, false)?;
            fs::File::open(&request.file)
                .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(request.file.clone()))?;
            Ok(PrServiceFileGuard::Import(identity))
        }
    }
}

fn revalidate_pr_service_file(
    request: &PrServiceRequest,
    expected: &PrServiceFileGuard,
) -> Result<(), MaintenanceSstateAdapterError> {
    let current = inspect_pr_service_file(request)?;
    if &current != expected {
        return Err(MaintenanceSstateAdapterError::StaleIdentity(
            request.file.clone(),
        ));
    }
    Ok(())
}

fn indexed_arguments(executable: &Path, arguments: &[String]) -> Vec<String> {
    std::iter::once(format!("0: {}", executable.display()))
        .chain(
            arguments
                .iter()
                .enumerate()
                .map(|(index, argument)| format!("{}: {argument}", index + 1)),
        )
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceSstateCommandKind {
    Readiness,
    CleanupPreview,
    CleanupExecute,
    PrServiceExport,
    PrServiceImport,
    LockedSignatureCache,
    BuildHistoryComparison,
    BuildCompare,
    GitArchiveLocal,
    GitArchivePush,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FilesystemIdentity {
    path: PathBuf,
    byte_size: u64,
    modified_at: std::time::SystemTime,
    directory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MaintenanceFilesystemGuard {
    Existing(FilesystemIdentity),
    Absent {
        path: PathBuf,
        parent: FilesystemIdentity,
    },
}

pub(crate) struct MaintenanceExternalCommand {
    pub session: MaintenanceSessionId,
    pub kind: MaintenanceSstateCommandKind,
    pub executable_identity: MaintenanceFileIdentity,
    pub expected_executable_name: String,
    pub arguments: Vec<OsString>,
    pub current_directory: PathBuf,
    pub timeout: Duration,
    pub preview: MaintenanceOperationPreview,
    pub guards: Vec<MaintenanceFilesystemGuard>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PrServiceFileGuard {
    Export {
        parent: FilesystemIdentity,
        existing: Option<FilesystemIdentity>,
    },
    Import(FilesystemIdentity),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceSstateCommandSpec {
    id: MaintenanceSessionId,
    kind: MaintenanceSstateCommandKind,
    executable_identity: MaintenanceFileIdentity,
    expected_executable_name: String,
    interface: MaintenanceToolInterface,
    arguments: Vec<OsString>,
    environment: BTreeMap<OsString, OsString>,
    current_directory: PathBuf,
    timeout: Duration,
    stdin_payload: Option<Vec<u8>>,
    preview: Option<MaintenanceOperationPreview>,
    cleanup_request: Option<SstateCleanupRequest>,
    cleanup_candidates: Vec<MaintenanceFileIdentity>,
    pr_service_guard: Option<PrServiceFileGuard>,
    external_guards: Vec<MaintenanceFilesystemGuard>,
}

impl MaintenanceSstateCommandSpec {
    pub fn readiness(
        session: MaintenanceSessionId,
        capability_request: u64,
        snapshot: &MaintenanceCapabilitySnapshot,
        operation_id: u64,
        request: SstateReadinessRequest,
    ) -> Result<(MaintenanceOperationPreview, Self), MaintenanceSstateAdapterError> {
        let (executable, interface) = available_tool(snapshot, MaintenanceTool::OeCheckSstate)?;
        if interface != MaintenanceToolInterface::Native {
            return Err(MaintenanceSstateAdapterError::Unavailable(
                "unsupported oe-check-sstate interface".into(),
            ));
        }
        revalidate_executable(executable, "oe-check-sstate")?;
        let build_dir = snapshot
            .metadata
            .build_dir
            .as_deref()
            .ok_or_else(|| {
                MaintenanceSstateAdapterError::Unavailable("BUILDDIR is unavailable".into())
            })
            .and_then(canonical_directory)?;
        for path in [request.output.as_deref(), request.log.as_deref()]
            .into_iter()
            .flatten()
        {
            validate_output_path(path)?;
        }
        if Duration::from_secs(request.timeout_seconds) > SSTATE_OPERATION_TIMEOUT {
            return Err(MaintenanceSstateAdapterError::InvalidInput(
                "readiness timeout exceeds the one-hour adapter limit".into(),
            ));
        }
        let timeout = Duration::from_secs(request.timeout_seconds);
        let arguments = readiness_arguments(&request);
        let indexed = indexed_arguments(&executable.path, &arguments);
        let preview = MaintenanceOperationPreview::new(
            operation_id,
            capability_request,
            MaintenanceOperation::SstateReadiness(request),
            indexed,
            Vec::new(),
        )
        .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))?;
        let mut environment = BTreeMap::new();
        environment.insert("BB_SETSCENE_ENFORCE".into(), "1".into());
        environment.insert("BUILDDIR".into(), build_dir.as_os_str().into());
        Ok((
            preview.clone(),
            Self {
                id: session,
                kind: MaintenanceSstateCommandKind::Readiness,
                executable_identity: executable.clone(),
                expected_executable_name: "oe-check-sstate".into(),
                interface,
                arguments: arguments.iter().map(OsString::from).collect(),
                environment,
                current_directory: build_dir,
                timeout,
                stdin_payload: None,
                preview: Some(preview),
                cleanup_request: None,
                cleanup_candidates: Vec::new(),
                pr_service_guard: None,
                external_guards: Vec::new(),
            },
        ))
    }

    pub fn cleanup_preview(
        session: MaintenanceSessionId,
        snapshot: &MaintenanceCapabilitySnapshot,
        request: SstateCleanupRequest,
    ) -> Result<Self, MaintenanceSstateAdapterError> {
        let (executable, interface) =
            available_tool(snapshot, MaintenanceTool::SstateCacheManagement)?;
        if !matches!(
            interface,
            MaintenanceToolInterface::SstatePython | MaintenanceToolInterface::SstateLegacyShell
        ) {
            return Err(MaintenanceSstateAdapterError::Unavailable(
                "unsupported sstate cleanup interface".into(),
            ));
        }
        let name = expected_name(interface, &executable.path).ok_or_else(|| {
            MaintenanceSstateAdapterError::Unavailable(
                "cleanup interface has no executable name".into(),
            )
        })?;
        revalidate_executable(executable, name)?;
        validate_cleanup_request(snapshot, &request)?;
        let arguments = cleanup_arguments(&request, true, false);
        Ok(Self {
            id: session,
            kind: MaintenanceSstateCommandKind::CleanupPreview,
            executable_identity: executable.clone(),
            expected_executable_name: name.into(),
            interface,
            arguments: arguments.iter().map(OsString::from).collect(),
            environment: BTreeMap::new(),
            current_directory: snapshot.metadata.build_dir.clone().ok_or_else(|| {
                MaintenanceSstateAdapterError::Unavailable("BUILDDIR is unavailable".into())
            })?,
            timeout: SSTATE_OPERATION_TIMEOUT,
            stdin_payload: Some(b"n\n".to_vec()),
            preview: None,
            cleanup_request: Some(request),
            cleanup_candidates: Vec::new(),
            pr_service_guard: None,
            external_guards: Vec::new(),
        })
    }

    pub fn cleanup_execution(
        session: MaintenanceSessionId,
        capability_request: u64,
        snapshot: &MaintenanceCapabilitySnapshot,
        operation_id: u64,
        confirmed: &SstateCleanupPreview,
        fresh: &SstateCleanupPreview,
    ) -> Result<(MaintenanceOperationPreview, Self), MaintenanceSstateAdapterError> {
        if confirmed != fresh {
            return Err(MaintenanceSstateAdapterError::CandidateMismatch);
        }
        let preview_command = Self::cleanup_preview(session, snapshot, confirmed.request.clone())?;
        for candidate in &confirmed.candidates {
            let current = identity_for_candidate(&candidate.path, &confirmed.request.cache_dir)?;
            if &current != candidate {
                return Err(MaintenanceSstateAdapterError::StaleIdentity(
                    candidate.path.clone(),
                ));
            }
        }
        let arguments = cleanup_arguments(&confirmed.request, false, true);
        let indexed = indexed_arguments(&preview_command.executable_identity.path, &arguments);
        let preview = MaintenanceOperationPreview::new(
            operation_id,
            capability_request,
            MaintenanceOperation::SstateCleanup(confirmed.clone()),
            indexed,
            Vec::new(),
        )
        .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))?;
        Ok((
            preview.clone(),
            Self {
                id: session,
                kind: MaintenanceSstateCommandKind::CleanupExecute,
                executable_identity: preview_command.executable_identity,
                expected_executable_name: preview_command.expected_executable_name,
                interface: preview_command.interface,
                arguments: arguments.iter().map(OsString::from).collect(),
                environment: BTreeMap::new(),
                current_directory: preview_command.current_directory,
                timeout: SSTATE_OPERATION_TIMEOUT,
                stdin_payload: None,
                preview: Some(preview),
                cleanup_request: Some(confirmed.request.clone()),
                cleanup_candidates: confirmed.candidates.clone(),
                pr_service_guard: None,
                external_guards: Vec::new(),
            },
        ))
    }

    pub fn pr_service(
        session: MaintenanceSessionId,
        capability_request: u64,
        snapshot: &MaintenanceCapabilitySnapshot,
        operation_id: u64,
        request: PrServiceRequest,
    ) -> Result<(MaintenanceOperationPreview, Self), MaintenanceSstateAdapterError> {
        let (executable, interface) = available_tool(snapshot, MaintenanceTool::PrServiceTool)?;
        if interface != MaintenanceToolInterface::Native {
            return Err(MaintenanceSstateAdapterError::Unavailable(
                "unsupported bitbake-prserv-tool interface".into(),
            ));
        }
        revalidate_executable(executable, "bitbake-prserv-tool")?;
        let build_dir = snapshot
            .metadata
            .build_dir
            .as_deref()
            .ok_or_else(|| {
                MaintenanceSstateAdapterError::Unavailable("BUILDDIR is unavailable".into())
            })
            .and_then(canonical_directory)?;
        if request.build_dir != build_dir
            || snapshot.metadata.prserv_host.as_ref() != Some(&request.endpoint)
        {
            return Err(MaintenanceSstateAdapterError::PreviewMismatch);
        }
        let guard = inspect_pr_service_file(&request)?;
        let arguments = pr_service_arguments(&request);
        let indexed = indexed_arguments(&executable.path, &arguments);
        let mut limitations = vec![
            format!("build directory: {}", build_dir.display()),
            format!("configured PR endpoint: {}", request.endpoint),
            "the native helper stops any active memory-resident BitBake server".into(),
            "the native helper invalidates BitBake cache records before parsing".into(),
        ];
        match request.operation {
            PrServiceOperation::Export => limitations
                .push("export may replace the exact selected .conf or .inc destination".into()),
            PrServiceOperation::Import => limitations.push("import changes PR service data".into()),
        }
        let preview = MaintenanceOperationPreview::new(
            operation_id,
            capability_request,
            MaintenanceOperation::PrService(request.clone()),
            indexed,
            limitations,
        )
        .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))?;
        Ok((
            preview.clone(),
            Self {
                id: session,
                kind: match request.operation {
                    PrServiceOperation::Export => MaintenanceSstateCommandKind::PrServiceExport,
                    PrServiceOperation::Import => MaintenanceSstateCommandKind::PrServiceImport,
                },
                executable_identity: executable.clone(),
                expected_executable_name: "bitbake-prserv-tool".into(),
                interface,
                arguments: arguments.iter().map(OsString::from).collect(),
                environment: BTreeMap::new(),
                current_directory: build_dir,
                timeout: SSTATE_OPERATION_TIMEOUT,
                stdin_payload: None,
                preview: Some(preview),
                cleanup_request: None,
                cleanup_candidates: Vec::new(),
                pr_service_guard: Some(guard),
                external_guards: Vec::new(),
            },
        ))
    }

    pub(crate) fn external(
        command: MaintenanceExternalCommand,
    ) -> Result<Self, MaintenanceSstateAdapterError> {
        if !matches!(
            command.kind,
            MaintenanceSstateCommandKind::LockedSignatureCache
                | MaintenanceSstateCommandKind::BuildHistoryComparison
                | MaintenanceSstateCommandKind::BuildCompare
                | MaintenanceSstateCommandKind::GitArchiveLocal
                | MaintenanceSstateCommandKind::GitArchivePush
        ) || command.expected_executable_name.is_empty()
            || command.timeout.is_zero()
        {
            return Err(MaintenanceSstateAdapterError::InvalidInput(
                "external Maintenance command is invalid".into(),
            ));
        }
        Ok(Self {
            id: command.session,
            kind: command.kind,
            executable_identity: command.executable_identity,
            expected_executable_name: command.expected_executable_name,
            interface: MaintenanceToolInterface::Native,
            arguments: command.arguments,
            environment: BTreeMap::new(),
            current_directory: command.current_directory,
            timeout: command.timeout,
            stdin_payload: None,
            preview: Some(command.preview),
            cleanup_request: None,
            cleanup_candidates: Vec::new(),
            pr_service_guard: None,
            external_guards: command.guards,
        })
    }

    pub fn id(&self) -> MaintenanceSessionId {
        self.id
    }

    pub fn kind(&self) -> MaintenanceSstateCommandKind {
        self.kind
    }

    pub fn executable(&self) -> &Path {
        &self.executable_identity.path
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn environment(&self) -> &BTreeMap<OsString, OsString> {
        &self.environment
    }

    pub fn current_directory(&self) -> &Path {
        &self.current_directory
    }

    fn revalidate(&self) -> Result<(), MaintenanceSstateAdapterError> {
        revalidate_executable(&self.executable_identity, &self.expected_executable_name)?;
        canonical_directory(&self.current_directory)?;
        if self.timeout.is_zero() {
            return Err(MaintenanceSstateAdapterError::InvalidInput(
                "operation timeout must be nonzero".into(),
            ));
        }
        if let Some(request) = &self.cleanup_request {
            let metadata = MaintenanceMetadata::new(MaintenanceMetadata {
                build_dir: Some(self.current_directory.clone()),
                sstate_dir: Some(request.cache_dir.clone()),
                stamps_dirs: request.stamps_dirs.clone(),
                ..MaintenanceMetadata::default()
            })
            .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))?;
            let snapshot = MaintenanceCapabilitySnapshot::new(
                metadata,
                vec![MaintenanceToolCapability::Available {
                    tool: MaintenanceTool::SstateCacheManagement,
                    executable: self.executable_identity.clone(),
                    interface: self.interface,
                }],
                vec![],
            )
            .map_err(|message| MaintenanceSstateAdapterError::InvalidInput(message.into()))?;
            validate_cleanup_request(&snapshot, request)?;
            for candidate in &self.cleanup_candidates {
                let current = identity_for_candidate(&candidate.path, &request.cache_dir)?;
                if &current != candidate {
                    return Err(MaintenanceSstateAdapterError::StaleIdentity(
                        candidate.path.clone(),
                    ));
                }
            }
        }
        for guard in &self.external_guards {
            revalidate_filesystem_guard(guard)?;
        }
        if let Some(preview) = &self.preview {
            let arguments = self
                .arguments
                .iter()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            if preview.arguments != indexed_arguments(&self.executable_identity.path, &arguments) {
                return Err(MaintenanceSstateAdapterError::PreviewMismatch);
            }
            match &preview.operation {
                MaintenanceOperation::SstateReadiness(request) => {
                    for path in [request.output.as_deref(), request.log.as_deref()]
                        .into_iter()
                        .flatten()
                    {
                        validate_output_path(path)?;
                    }
                    if arguments != readiness_arguments(request)
                        || self.timeout != Duration::from_secs(request.timeout_seconds)
                        || self.environment.get(OsStr::new("BB_SETSCENE_ENFORCE"))
                            != Some(&OsString::from("1"))
                    {
                        return Err(MaintenanceSstateAdapterError::PreviewMismatch);
                    }
                }
                MaintenanceOperation::SstateCleanup(cleanup) => {
                    if self.kind != MaintenanceSstateCommandKind::CleanupExecute
                        || arguments != cleanup_arguments(&cleanup.request, false, true)
                        || self.stdin_payload.is_some()
                    {
                        return Err(MaintenanceSstateAdapterError::PreviewMismatch);
                    }
                }
                MaintenanceOperation::PrService(request) => {
                    let expected_kind = match request.operation {
                        PrServiceOperation::Export => MaintenanceSstateCommandKind::PrServiceExport,
                        PrServiceOperation::Import => MaintenanceSstateCommandKind::PrServiceImport,
                    };
                    if self.kind != expected_kind
                        || arguments != pr_service_arguments(request)
                        || request.build_dir != self.current_directory
                        || self.stdin_payload.is_some()
                    {
                        return Err(MaintenanceSstateAdapterError::PreviewMismatch);
                    }
                    let guard = self
                        .pr_service_guard
                        .as_ref()
                        .ok_or(MaintenanceSstateAdapterError::PreviewMismatch)?;
                    revalidate_pr_service_file(request, guard)?;
                }
                MaintenanceOperation::LockedSignatureCache(_)
                    if self.kind == MaintenanceSstateCommandKind::LockedSignatureCache => {}
                MaintenanceOperation::BuildHistoryComparison(_)
                    if self.kind == MaintenanceSstateCommandKind::BuildHistoryComparison => {}
                MaintenanceOperation::BuildCompare(_)
                    if self.kind == MaintenanceSstateCommandKind::BuildCompare => {}
                MaintenanceOperation::GitArchive(request) => {
                    let expected_kind = if request.push_remote.is_some() {
                        MaintenanceSstateCommandKind::GitArchivePush
                    } else {
                        MaintenanceSstateCommandKind::GitArchiveLocal
                    };
                    if self.kind != expected_kind {
                        return Err(MaintenanceSstateAdapterError::PreviewMismatch);
                    }
                }
                _ => return Err(MaintenanceSstateAdapterError::PreviewMismatch),
            }
        } else if let Some(request) = &self.cleanup_request {
            let arguments = self
                .arguments
                .iter()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            if self.kind != MaintenanceSstateCommandKind::CleanupPreview
                || arguments != cleanup_arguments(request, true, false)
                || self.stdin_payload.as_deref() != Some(b"n\n")
            {
                return Err(MaintenanceSstateAdapterError::PreviewMismatch);
            }
        }
        Ok(())
    }
}

fn validate_cleanup_request(
    snapshot: &MaintenanceCapabilitySnapshot,
    request: &SstateCleanupRequest,
) -> Result<(), MaintenanceSstateAdapterError> {
    let cache = canonical_directory(&request.cache_dir)?;
    if snapshot.metadata.sstate_dir.as_ref() != Some(&cache) {
        return Err(MaintenanceSstateAdapterError::PreviewMismatch);
    }
    let mut stamps = request
        .stamps_dirs
        .iter()
        .map(|path| canonical_directory(path))
        .collect::<Result<Vec<_>, _>>()?;
    stamps.sort();
    stamps.dedup();
    if stamps != request.stamps_dirs
        || stamps
            .iter()
            .any(|path| !snapshot.metadata.stamps_dirs.contains(path))
    {
        return Err(MaintenanceSstateAdapterError::PreviewMismatch);
    }
    Ok(())
}

pub fn parse_cleanup_preview(
    request: SstateCleanupRequest,
    lines: &[String],
) -> Result<SstateCleanupPreview, MaintenanceSstateAdapterError> {
    if lines.iter().map(String::len).sum::<usize>() > MAX_PREVIEW_OUTPUT_BYTES {
        return Err(MaintenanceSstateAdapterError::InvalidPreviewOutput(
            "preview output exceeded the byte limit".into(),
        ));
    }
    let mut candidates = Vec::new();
    for line in lines.iter().take(MAX_MAINTENANCE_PATHS + 1) {
        let path = Path::new(line.trim());
        if !path.is_absolute() {
            continue;
        }
        candidates.push(identity_for_candidate(path, &request.cache_dir)?);
    }
    if candidates.len() > MAX_MAINTENANCE_PATHS {
        return Err(MaintenanceSstateAdapterError::InvalidPreviewOutput(
            "preview candidate count exceeded the limit".into(),
        ));
    }
    SstateCleanupPreview::new(request, candidates)
        .map_err(|message| MaintenanceSstateAdapterError::InvalidPreviewOutput(message.into()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceSstateRunnerEvent {
    Started {
        id: MaintenanceSessionId,
    },
    Output {
        id: MaintenanceSessionId,
        stream: MaintenanceOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        id: MaintenanceSessionId,
        exit_code: Option<i32>,
    },
    Failed {
        id: MaintenanceSessionId,
        exit_code: Option<i32>,
    },
    CancellationRequested {
        id: MaintenanceSessionId,
    },
    Cancelled {
        id: MaintenanceSessionId,
        forced: bool,
        exit_code: Option<i32>,
    },
    CancellationRejected {
        id: MaintenanceSessionId,
        message: String,
    },
    TimedOut {
        id: MaintenanceSessionId,
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        id: MaintenanceSessionId,
        message: String,
    },
}

#[derive(Debug)]
enum PipeEvent {
    Output {
        stream: MaintenanceOutputStream,
        line: String,
        truncated: bool,
    },
    Failed {
        stream: MaintenanceOutputStream,
        message: String,
    },
}

async fn read_output<R>(
    stream: R,
    kind: MaintenanceOutputStream,
    sender: tokio::sync::mpsc::Sender<PipeEvent>,
) where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut bytes = Vec::new();
    let mut truncated = false;
    loop {
        let buffer = match reader.fill_buf().await {
            Ok(buffer) => buffer,
            Err(error) => {
                let _ = sender
                    .send(PipeEvent::Failed {
                        stream: kind,
                        message: error.to_string(),
                    })
                    .await;
                break;
            }
        };
        if buffer.is_empty() {
            if !bytes.is_empty() || truncated {
                let _ = sender
                    .send(PipeEvent::Output {
                        stream: kind,
                        line: output_text(&bytes),
                        truncated,
                    })
                    .await;
            }
            break;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(buffer.len());
        if !truncated {
            let remaining = MAX_MAINTENANCE_TEXT_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(PipeEvent::Output {
                    stream: kind,
                    line: output_text(&bytes),
                    truncated,
                })
                .await
                .is_err()
            {
                break;
            }
            bytes.clear();
            truncated = false;
        }
    }
}

fn output_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\r')
        .to_string()
}

pub struct MaintenanceSstateJobRunner {
    id: Option<MaintenanceSessionId>,
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<PipeEvent>>,
    streams_drained: bool,
    started_pending: bool,
    terminal_pending: VecDeque<MaintenanceSstateRunnerEvent>,
    cancellation_timeout: Duration,
    operation_timeout: Duration,
    deadline: Option<Instant>,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl Default for MaintenanceSstateJobRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl MaintenanceSstateJobRunner {
    pub fn new() -> Self {
        Self {
            id: None,
            child: None,
            output: None,
            streams_drained: true,
            started_pending: false,
            terminal_pending: VecDeque::new(),
            cancellation_timeout: Duration::from_secs(5),
            operation_timeout: SSTATE_OPERATION_TIMEOUT,
            deadline: None,
            cancellation_requested: false,
            #[cfg(unix)]
            process_group: None,
        }
    }

    pub fn with_cancellation_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_timeout = timeout;
        self
    }

    pub fn with_operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    pub fn is_active(&self) -> bool {
        self.child.is_some()
    }

    pub async fn start(
        &mut self,
        command: MaintenanceSstateCommandSpec,
    ) -> Result<(), MaintenanceSstateAdapterError> {
        if self.child.is_some()
            || self.started_pending
            || !self.terminal_pending.is_empty()
            || self.output.is_some()
        {
            return Err(MaintenanceSstateAdapterError::Busy);
        }
        command.revalidate()?;
        let id = command.id();
        let timeout = command.timeout;
        let stdin_payload = command.stdin_payload.clone();
        let mut process = Command::new(command.executable());
        process
            .args(command.arguments())
            .envs(command.environment())
            .current_dir(command.current_directory())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if stdin_payload.is_some() {
            process.stdin(Stdio::piped());
        } else {
            process.stdin(Stdio::null());
        }
        #[cfg(unix)]
        process.process_group(0);
        let mut child = process
            .spawn()
            .map_err(|error| MaintenanceSstateAdapterError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        if let Some(payload) = stdin_payload {
            let Some(mut stdin) = child.stdin.take() else {
                let _ = child.kill().await;
                self.clear_process_state();
                return Err(MaintenanceSstateAdapterError::ProcessControl(
                    "cleanup preview stdin is unavailable".into(),
                ));
            };
            if let Err(error) = stdin.write_all(&payload).await {
                let _ = child.kill().await;
                let _ = child.wait().await;
                self.clear_process_state();
                return Err(MaintenanceSstateAdapterError::ProcessControl(
                    error.to_string(),
                ));
            }
            drop(stdin);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            self.clear_process_state();
            return Err(MaintenanceSstateAdapterError::StreamUnavailable(
                MaintenanceOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill().await;
            self.clear_process_state();
            return Err(MaintenanceSstateAdapterError::StreamUnavailable(
                MaintenanceOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(SSTATE_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_output(
            stdout,
            MaintenanceOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_output(
            stderr,
            MaintenanceOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.id = Some(id);
        self.child = Some(child);
        self.output = Some(receiver);
        self.streams_drained = false;
        self.started_pending = true;
        self.deadline = Some(Instant::now() + timeout.min(self.operation_timeout));
        self.cancellation_requested = false;
        Ok(())
    }

    pub async fn next_event(
        &mut self,
    ) -> Result<MaintenanceSstateRunnerEvent, MaintenanceSstateAdapterError> {
        if let Some(event) = self.terminal_pending.pop_front() {
            return Ok(event);
        }
        let id = self.id.ok_or(MaintenanceSstateAdapterError::NotRunning)?;
        if self.started_pending {
            self.started_pending = false;
            return Ok(MaintenanceSstateRunnerEvent::Started { id });
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            self.kill_and_clear().await;
            return Ok(MaintenanceSstateRunnerEvent::Lost {
                id,
                message: "sstate output event channel was lost".into(),
            });
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return self.timeout_active(id).await;
        }
        if let Some(receiver) = self.output.as_mut() {
            let deadline = self
                .deadline
                .ok_or(MaintenanceSstateAdapterError::NotRunning)?;
            let event = tokio::select! {
                event = receiver.recv() => Some(event),
                _ = tokio::time::sleep_until(deadline) => None,
            };
            match event {
                Some(Some(PipeEvent::Output {
                    stream,
                    line,
                    truncated,
                })) => {
                    return Ok(MaintenanceSstateRunnerEvent::Output {
                        id,
                        stream,
                        line,
                        truncated,
                    });
                }
                Some(Some(PipeEvent::Failed { stream, message })) => {
                    self.kill_and_clear().await;
                    return Ok(MaintenanceSstateRunnerEvent::Lost {
                        id,
                        message: format!("{stream:?} stream failed: {message}"),
                    });
                }
                Some(None) => {
                    self.output = None;
                    self.streams_drained = true;
                }
                None => return self.timeout_active(id).await,
            }
        }
        let deadline = self
            .deadline
            .ok_or(MaintenanceSstateAdapterError::NotRunning)?;
        let status = {
            let child = self
                .child
                .as_mut()
                .ok_or(MaintenanceSstateAdapterError::NotRunning)?;
            match tokio::time::timeout_at(deadline, child.wait()).await {
                Ok(Ok(status)) => status,
                Ok(Err(error)) => {
                    self.kill_and_clear().await;
                    return Ok(MaintenanceSstateRunnerEvent::Lost {
                        id,
                        message: format!("sstate wait failed: {error}"),
                    });
                }
                Err(_) => return self.timeout_active(id).await,
            }
        };
        self.clear_process_state();
        if status.success() {
            Ok(MaintenanceSstateRunnerEvent::Completed {
                id,
                exit_code: status.code(),
            })
        } else {
            Ok(MaintenanceSstateRunnerEvent::Failed {
                id,
                exit_code: status.code(),
            })
        }
    }

    pub async fn cancel(
        &mut self,
        requested_id: MaintenanceSessionId,
    ) -> Result<bool, MaintenanceSstateAdapterError> {
        if self.cancellation_requested || self.child.is_none() || self.id != Some(requested_id) {
            self.terminal_pending
                .push_back(MaintenanceSstateRunnerEvent::CancellationRejected {
                    id: requested_id,
                    message: "no matching cancellable sstate process is active".into(),
                });
            return Ok(false);
        }
        self.cancellation_requested = true;
        self.terminal_pending
            .push_back(MaintenanceSstateRunnerEvent::CancellationRequested { id: requested_id });
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state_preserving_events();
        self.terminal_pending
            .push_back(MaintenanceSstateRunnerEvent::Cancelled {
                id: requested_id,
                forced,
                exit_code: status.and_then(|status| status.code()),
            });
        Ok(true)
    }

    async fn timeout_active(
        &mut self,
        id: MaintenanceSessionId,
    ) -> Result<MaintenanceSstateRunnerEvent, MaintenanceSstateAdapterError> {
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state();
        Ok(MaintenanceSstateRunnerEvent::TimedOut {
            id,
            forced,
            exit_code: status.and_then(|status| status.code()),
        })
    }

    async fn terminate_active(
        &mut self,
    ) -> Result<(Option<std::process::ExitStatus>, bool), MaintenanceSstateAdapterError> {
        let Some(child) = self.child.as_mut() else {
            return Ok((None, false));
        };
        let mut forced = false;
        #[cfg(unix)]
        let status = if let Some(process_group) = self.process_group {
            // SAFETY: the negative PID targets only the process group created for this child.
            if unsafe { libc::kill(-process_group, libc::SIGTERM) } != 0 {
                child.start_kill().map_err(|error| {
                    MaintenanceSstateAdapterError::ProcessControl(error.to_string())
                })?;
                forced = true;
            }
            match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                Ok(result) => Some(result.map_err(|error| {
                    MaintenanceSstateAdapterError::ProcessControl(error.to_string())
                })?),
                Err(_) => {
                    // SAFETY: same child-owned process group as the graceful signal.
                    let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                    forced = true;
                    Some(child.wait().await.map_err(|error| {
                        MaintenanceSstateAdapterError::ProcessControl(error.to_string())
                    })?)
                }
            }
        } else {
            forced = true;
            child.kill().await.map_err(|error| {
                MaintenanceSstateAdapterError::ProcessControl(error.to_string())
            })?;
            Some(child.wait().await.map_err(|error| {
                MaintenanceSstateAdapterError::ProcessControl(error.to_string())
            })?)
        };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child.kill().await.map_err(|error| {
                MaintenanceSstateAdapterError::ProcessControl(error.to_string())
            })?;
            Some(child.wait().await.map_err(|error| {
                MaintenanceSstateAdapterError::ProcessControl(error.to_string())
            })?)
        };
        Ok((status, forced))
    }

    async fn kill_and_clear(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.clear_process_state();
    }

    fn clear_process_state(&mut self) {
        self.clear_process_state_preserving_events();
        self.terminal_pending.clear();
    }

    fn clear_process_state_preserving_events(&mut self) {
        self.id = None;
        self.child = None;
        self.output = None;
        self.streams_drained = true;
        self.started_pending = false;
        self.deadline = None;
        self.cancellation_requested = false;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
    }

    #[cfg(test)]
    pub(crate) fn lose_output_channel(&mut self) {
        self.output = None;
    }
}

impl Drop for MaintenanceSstateJobRunner {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(process_group) = self.process_group {
            // SAFETY: this is the child-owned process group created by `start`.
            let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-maintenance-sstate-{name}-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(unix)]
    fn write_executable(path: &Path, body: &str) {
        fs::write(path, body).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    fn fixture(
        name: &str,
        cleanup_name: &str,
        body: &str,
    ) -> (TestDirectory, MaintenanceCapabilitySnapshot) {
        let root = TestDirectory::new(name);
        for directory in ["bin", "build", "cache", "tmp", "stamps", "output"] {
            fs::create_dir(root.0.join(directory)).unwrap();
        }
        write_executable(&root.0.join("bin/oe-check-sstate"), body);
        write_executable(&root.0.join("bin").join(cleanup_name), body);
        let snapshot =
            MaintenanceSstateCapabilityInspector::inspect(MaintenanceSstateCapabilityInput {
                build_dir: root.0.join("build"),
                sstate_dir: Some(root.0.join("cache")),
                tmp_dir: Some(root.0.join("tmp")),
                stamps_dirs: vec![root.0.join("stamps")],
                executable_search_path: vec![root.0.join("bin")],
            })
            .unwrap();
        (root, snapshot)
    }

    fn cleanup_request(root: &TestDirectory) -> SstateCleanupRequest {
        SstateCleanupRequest::new(
            root.0.join("cache"),
            vec![root.0.join("stamps")],
            vec![
                SstateCleanupMode::Duplicates,
                SstateCleanupMode::Orphans,
                SstateCleanupMode::UnreferencedByStamps,
            ],
            4,
        )
        .unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn maintenance_sstate_capability_distinguishes_python_legacy_missing_and_unsafe() {
        let (root, snapshot) = fixture(
            "python-capability",
            "sstate-cache-management.py",
            "#!/bin/sh\nexit 0\n",
        );
        assert!(matches!(
            snapshot.capability(MaintenanceTool::SstateCacheManagement),
            Some(MaintenanceToolCapability::Available {
                interface: MaintenanceToolInterface::SstatePython,
                ..
            })
        ));
        fs::remove_file(root.0.join("bin/sstate-cache-management.py")).unwrap();
        write_executable(
            &root.0.join("bin/sstate-cache-management.sh"),
            "#!/bin/sh\nexit 0\n",
        );
        let legacy =
            MaintenanceSstateCapabilityInspector::inspect(MaintenanceSstateCapabilityInput {
                build_dir: root.0.join("build"),
                sstate_dir: Some(root.0.join("cache")),
                tmp_dir: Some(root.0.join("tmp")),
                stamps_dirs: vec![root.0.join("stamps")],
                executable_search_path: vec![root.0.join("bin")],
            })
            .unwrap();
        assert!(matches!(
            legacy.capability(MaintenanceTool::SstateCacheManagement),
            Some(MaintenanceToolCapability::Available {
                interface: MaintenanceToolInterface::SstateLegacyShell,
                ..
            })
        ));
        fs::remove_file(root.0.join("bin/sstate-cache-management.sh")).unwrap();
        let missing =
            MaintenanceSstateCapabilityInspector::inspect(MaintenanceSstateCapabilityInput {
                build_dir: root.0.join("build"),
                sstate_dir: Some(root.0.join("cache")),
                tmp_dir: None,
                stamps_dirs: vec![],
                executable_search_path: vec![root.0.join("bin")],
            })
            .unwrap();
        assert!(matches!(
            missing.capability(MaintenanceTool::SstateCacheManagement),
            Some(MaintenanceToolCapability::Unavailable { .. })
        ));

        let linked = root.0.join("linked-bin");
        symlink(root.0.join("bin"), &linked).unwrap();
        let unsafe_snapshot =
            MaintenanceSstateCapabilityInspector::inspect(MaintenanceSstateCapabilityInput {
                build_dir: root.0.join("build"),
                sstate_dir: Some(root.0.join("cache")),
                tmp_dir: None,
                stamps_dirs: vec![],
                executable_search_path: vec![linked],
            })
            .unwrap();
        assert!(!unsafe_snapshot.limitations.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn maintenance_sstate_reconstructs_exact_readiness_and_cleanup_vectors() {
        let (root, snapshot) = fixture(
            "vectors",
            "sstate-cache-management.py",
            "#!/bin/sh\nexit 0\n",
        );
        let output = root.0.join("output/readiness.txt");
        let (preview, command) = MaintenanceSstateCommandSpec::readiness(
            MaintenanceSessionId(1),
            3,
            &snapshot,
            9,
            SstateReadinessRequest::new(
                vec!["busybox".into(), "core-image-minimal".into()],
                SstateReadinessMode::SameTmpdir,
                Some(output.clone()),
                None,
                60,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            command.arguments(),
            &[
                OsString::from("--outfile"),
                output.into_os_string(),
                OsString::from("--same-tmpdir"),
                OsString::from("busybox"),
                OsString::from("core-image-minimal"),
            ]
        );
        assert_eq!(
            command.environment().get(OsStr::new("BB_SETSCENE_ENFORCE")),
            Some(&OsString::from("1"))
        );
        assert_eq!(
            preview.arguments[0],
            format!(
                "0: {}",
                snapshot
                    .capability(MaintenanceTool::OeCheckSstate)
                    .and_then(|capability| match capability {
                        MaintenanceToolCapability::Available { executable, .. } =>
                            Some(executable.path.display().to_string()),
                        _ => None,
                    })
                    .unwrap()
            )
        );

        let cleanup = MaintenanceSstateCommandSpec::cleanup_preview(
            MaintenanceSessionId(2),
            &snapshot,
            cleanup_request(&root),
        )
        .unwrap();
        let arguments = cleanup
            .arguments()
            .iter()
            .map(|value| value.to_string_lossy())
            .collect::<Vec<_>>();
        assert!(arguments.contains(&std::borrow::Cow::Borrowed("--remove-duplicated")));
        assert!(arguments.contains(&std::borrow::Cow::Borrowed("--remove-orphans")));
        assert!(arguments.contains(&std::borrow::Cow::Borrowed("--stamps-dir")));
        assert_eq!(
            arguments.last().map(|value| value.as_ref()),
            Some("--debug")
        );
    }

    #[cfg(unix)]
    #[test]
    fn maintenance_sstate_preview_is_bounded_and_cleanup_rejects_changes_or_tampering() {
        let (root, snapshot) = fixture(
            "preview",
            "sstate-cache-management.py",
            "#!/bin/sh\nexit 0\n",
        );
        let candidate = root.0.join("cache/sstate:a");
        fs::write(&candidate, "one").unwrap();
        let request = cleanup_request(&root);
        let confirmed =
            parse_cleanup_preview(request.clone(), &[candidate.display().to_string()]).unwrap();
        assert_eq!(confirmed.candidates.len(), 1);
        assert!(
            parse_cleanup_preview(request.clone(), &[])
                .unwrap()
                .candidates
                .is_empty()
        );
        let (execution_preview, execution) = MaintenanceSstateCommandSpec::cleanup_execution(
            MaintenanceSessionId(3),
            1,
            &snapshot,
            3,
            &confirmed,
            &confirmed,
        )
        .unwrap();
        assert_eq!(execution.arguments().last(), Some(&OsString::from("--yes")));
        assert!(
            !execution
                .arguments()
                .iter()
                .any(|argument| argument == OsStr::new("--debug"))
        );
        assert_eq!(
            execution_preview.operation,
            MaintenanceOperation::SstateCleanup(confirmed.clone())
        );
        let changed = SstateCleanupPreview::new(request.clone(), vec![]).unwrap();
        assert!(matches!(
            MaintenanceSstateCommandSpec::cleanup_execution(
                MaintenanceSessionId(3),
                1,
                &snapshot,
                3,
                &confirmed,
                &changed,
            ),
            Err(MaintenanceSstateAdapterError::CandidateMismatch)
        ));
        fs::write(&candidate, "changed").unwrap();
        assert!(matches!(
            MaintenanceSstateCommandSpec::cleanup_execution(
                MaintenanceSessionId(3),
                1,
                &snapshot,
                3,
                &confirmed,
                &confirmed,
            ),
            Err(MaintenanceSstateAdapterError::StaleIdentity(path)) if path == candidate
        ));
        assert!(
            parse_cleanup_preview(
                request,
                &[format!("/outside/{}", "x".repeat(MAX_PREVIEW_OUTPUT_BYTES))]
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn maintenance_sstate_runner_streams_bounded_output_and_terminal_status() {
        let (_root, snapshot) = fixture(
            "runner",
            "sstate-cache-management.py",
            &format!(
                "#!/bin/sh\nprintf 'out\\n'\nprintf '%*s\\n' {} x >&2\nexit 0\n",
                MAX_MAINTENANCE_TEXT_BYTES + 64
            ),
        );
        let (_, command) = MaintenanceSstateCommandSpec::readiness(
            MaintenanceSessionId(4),
            1,
            &snapshot,
            4,
            SstateReadinessRequest::new(
                vec!["busybox".into()],
                SstateReadinessMode::IsolatedTmpdir,
                None,
                None,
                60,
            )
            .unwrap(),
        )
        .unwrap();
        let mut runner = MaintenanceSstateJobRunner::new();
        runner.start(command).await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Started { .. }
        ));
        let mut truncated = false;
        loop {
            match runner.next_event().await.unwrap() {
                MaintenanceSstateRunnerEvent::Output {
                    truncated: value, ..
                } => truncated |= value,
                MaintenanceSstateRunnerEvent::Completed {
                    exit_code: Some(0), ..
                } => break,
                event => panic!("unexpected event {event:?}"),
            }
        }
        assert!(truncated);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn maintenance_sstate_cleanup_preview_receives_negative_input_and_revalidates_tool() {
        let (root, snapshot) = fixture(
            "preview-runner",
            "sstate-cache-management.py",
            "#!/bin/sh\nread answer\nprintf '%s\\n' \"$answer\"\nexit 0\n",
        );
        let command = MaintenanceSstateCommandSpec::cleanup_preview(
            MaintenanceSessionId(40),
            &snapshot,
            cleanup_request(&root),
        )
        .unwrap();
        let mut runner = MaintenanceSstateJobRunner::new();
        runner.start(command).await.unwrap();
        runner.next_event().await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Output {
                stream: MaintenanceOutputStream::Stdout,
                line,
                ..
            } if line == "n"
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Completed {
                exit_code: Some(0),
                ..
            }
        ));

        let command = MaintenanceSstateCommandSpec::cleanup_preview(
            MaintenanceSessionId(41),
            &snapshot,
            cleanup_request(&root),
        )
        .unwrap();
        write_executable(
            &root.0.join("bin/sstate-cache-management.py"),
            "#!/bin/sh\nprintf 'tampered\\n'\nexit 0\n",
        );
        assert!(matches!(
            MaintenanceSstateJobRunner::new().start(command).await,
            Err(MaintenanceSstateAdapterError::StaleIdentity(_))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn maintenance_sstate_runner_reports_nonzero_duplicate_and_cancellation() {
        let (_root, snapshot) = fixture(
            "cancel",
            "sstate-cache-management.py",
            "#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n",
        );
        let (_, command) = MaintenanceSstateCommandSpec::readiness(
            MaintenanceSessionId(5),
            1,
            &snapshot,
            5,
            SstateReadinessRequest::new(
                vec!["busybox".into()],
                SstateReadinessMode::IsolatedTmpdir,
                None,
                None,
                60,
            )
            .unwrap(),
        )
        .unwrap();
        let mut runner = MaintenanceSstateJobRunner::new();
        runner.start(command.clone()).await.unwrap();
        assert!(matches!(
            runner.start(command).await,
            Err(MaintenanceSstateAdapterError::Busy)
        ));
        assert!(runner.cancel(MaintenanceSessionId(5)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::CancellationRequested { .. }
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Cancelled { forced: false, .. }
        ));

        let (_root, snapshot) = fixture(
            "nonzero",
            "sstate-cache-management.py",
            "#!/bin/sh\nexit 7\n",
        );
        let (_, command) = MaintenanceSstateCommandSpec::readiness(
            MaintenanceSessionId(6),
            1,
            &snapshot,
            6,
            SstateReadinessRequest::new(
                vec!["busybox".into()],
                SstateReadinessMode::IsolatedTmpdir,
                None,
                None,
                60,
            )
            .unwrap(),
        )
        .unwrap();
        let mut runner = MaintenanceSstateJobRunner::new();
        runner.start(command).await.unwrap();
        runner.next_event().await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Failed {
                exit_code: Some(7),
                ..
            }
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn maintenance_sstate_runner_preserves_timeout_forced_cancel_rejection_and_loss() {
        let (_root, snapshot) = fixture(
            "timeout",
            "sstate-cache-management.py",
            "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
        );
        let make_command = |id| {
            MaintenanceSstateCommandSpec::readiness(
                MaintenanceSessionId(id),
                1,
                &snapshot,
                id,
                SstateReadinessRequest::new(
                    vec!["busybox".into()],
                    SstateReadinessMode::IsolatedTmpdir,
                    None,
                    None,
                    60,
                )
                .unwrap(),
            )
            .unwrap()
            .1
        };
        let mut runner = MaintenanceSstateJobRunner::new()
            .with_operation_timeout(Duration::from_millis(1))
            .with_cancellation_timeout(Duration::from_millis(1));
        runner.start(make_command(7)).await.unwrap();
        runner.next_event().await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::TimedOut { forced: true, .. }
        ));
        assert!(!runner.cancel(MaintenanceSessionId(99)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::CancellationRejected { .. }
        ));

        let mut runner = MaintenanceSstateJobRunner::new();
        runner.start(make_command(8)).await.unwrap();
        runner.next_event().await.unwrap();
        runner.lose_output_channel();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Lost { .. }
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn hardening_stress_process_tree_cancellation_reaps_descendant() {
        let (root, snapshot) = fixture(
            "process-tree-stress",
            "sstate-cache-management.py",
            "#!/bin/sh\n(\n  trap '' TERM\n  while :; do sleep 1; done\n) &\ndescendant=$!\nprintf '%s\\n' \"$descendant\" > descendant.pid\ntrap 'wait \"$descendant\"' TERM\nwhile :; do sleep 1; done\n",
        );
        let descendant_file = root.0.join("build/descendant.pid");
        let (_, command) = MaintenanceSstateCommandSpec::readiness(
            MaintenanceSessionId(70),
            1,
            &snapshot,
            70,
            SstateReadinessRequest::new(
                vec!["busybox".into()],
                SstateReadinessMode::IsolatedTmpdir,
                None,
                None,
                60,
            )
            .unwrap(),
        )
        .unwrap();
        let mut runner =
            MaintenanceSstateJobRunner::new().with_cancellation_timeout(Duration::from_millis(50));
        runner.start(command).await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Started { .. }
        ));

        for _ in 0..200 {
            if descendant_file.is_file() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let descendant = fs::read_to_string(&descendant_file)
            .unwrap()
            .trim()
            .parse::<i32>()
            .unwrap();
        // SAFETY: signal zero only probes the exact child PID written by the fixture.
        assert_eq!(unsafe { libc::kill(descendant, 0) }, 0);

        assert!(runner.cancel(MaintenanceSessionId(70)).await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::CancellationRequested { .. }
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Cancelled { forced: true, .. }
        ));
        let mut reaped = false;
        for _ in 0..200 {
            // SAFETY: signal zero only probes the previously observed fixture PID.
            if unsafe { libc::kill(descendant, 0) } != 0 {
                reaped = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(reaped, "cancelled process-group descendant survived");
    }
}
