use std::{
    collections::{BTreeSet, VecDeque},
    ffi::{OsStr, OsString},
    fs, io,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
    time::Instant,
};
use yoctui_model::{
    MAX_QA_LAYER_ARGUMENTS, MAX_QA_REPORT_PATHS, MAX_QA_SCOPES, MAX_QA_TEXT_BYTES, QaCheckId,
    QaConfiguredLayerCapability, QaExecutableIdentity, QaLayerCapabilitySnapshot, QaLayerIdentity,
    QaLayerOperationId, QaLayerOperationPreview, QaLayerRunCapability, QaLayerSessionId,
    QaOutputStream,
};

use crate::output_text;

const QA_LAYER_EVENT_CHANNEL_CAPACITY: usize = 256;
const QA_LAYER_OPERATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const QA_LAYER_SPAWN_ATTEMPTS: usize = 4;
const QA_LAYER_SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);

#[cfg(unix)]
fn is_transient_qa_layer_spawn_error(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ETXTBSY)
}

#[cfg(not(unix))]
fn is_transient_qa_layer_spawn_error(_error: &io::Error) -> bool {
    false
}

async fn spawn_qa_layer_process(process: &mut Command) -> io::Result<Child> {
    for attempt in 1..=QA_LAYER_SPAWN_ATTEMPTS {
        match process.spawn() {
            Ok(child) => return Ok(child),
            Err(error)
                if attempt < QA_LAYER_SPAWN_ATTEMPTS
                    && is_transient_qa_layer_spawn_error(&error) =>
            {
                tokio::time::sleep(QA_LAYER_SPAWN_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the bounded layer-QA process spawn loop always returns")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaConfiguredLayerInput {
    pub check: QaCheckId,
    pub identity: QaLayerIdentity,
    pub compatible_series: Vec<String>,
    pub report_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaLayerCapabilityInput {
    pub release: Option<String>,
    pub build_directory: PathBuf,
    pub selected_layer: QaLayerIdentity,
    pub layers: Vec<QaConfiguredLayerInput>,
    pub executable_search_path: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaLayerCapabilityResponse {
    Available(QaLayerCapabilitySnapshot),
    Partial(QaLayerCapabilitySnapshot),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum QaLayerAdapterError {
    #[error("invalid layer-QA capability input: {0}")]
    InvalidInput(String),
    #[error("layer-QA preview is invalid: {0}")]
    InvalidPreview(String),
    #[error("layer-QA preview was modified after confirmation")]
    PreviewMismatch,
    #[error("layer-QA executable is unsafe: {0}")]
    UnsafeExecutable(PathBuf),
    #[error("configured layer identity is unsafe: {0}")]
    UnsafeLayer(PathBuf),
    #[error("layer-QA report root is unsafe: {0}")]
    UnsafeReportRoot(PathBuf),
    #[error("layer-QA identity became stale: {0}")]
    StaleIdentity(PathBuf),
    #[error("a layer-QA process or unconsumed event is already active")]
    Busy,
    #[error("could not start layer QA: {0}")]
    Spawn(String),
    #[error("layer-QA process stream is unavailable: {0:?}")]
    StreamUnavailable(QaOutputStream),
    #[error("layer-QA runner is not active")]
    NotRunning,
    #[error("layer-QA process control failed: {0}")]
    ProcessControl(String),
}

#[derive(Debug, Default)]
pub struct QaLayerCapabilityInspector;

impl QaLayerCapabilityInspector {
    pub fn inspect(
        input: QaLayerCapabilityInput,
    ) -> Result<QaLayerCapabilityResponse, QaLayerAdapterError> {
        if input.layers.is_empty()
            || input.layers.len() > MAX_QA_SCOPES
            || input.executable_search_path.len() > MAX_QA_REPORT_PATHS
            || !input.selected_layer.is_valid()
        {
            return Err(QaLayerAdapterError::InvalidInput(
                "configured layers, selected layer, or search path are invalid".into(),
            ));
        }
        let build_directory = canonical_directory(&input.build_directory)
            .map_err(|_| QaLayerAdapterError::InvalidInput("build directory is unsafe".into()))?;
        if build_directory != input.build_directory {
            return Err(QaLayerAdapterError::InvalidInput(
                "build directory must be canonical".into(),
            ));
        }
        let mut identities = BTreeSet::new();
        if input.layers.iter().any(|layer| {
            !layer.check.is_valid()
                || !layer.identity.is_valid()
                || !identities.insert(layer.identity.clone())
        }) || !input
            .layers
            .iter()
            .any(|layer| layer.identity == input.selected_layer)
        {
            return Err(QaLayerAdapterError::InvalidInput(
                "configured layer identities must be valid, unique, and include the selection"
                    .into(),
            ));
        }

        let mut limitations = Vec::new();
        let executable = discover_executable(&input.executable_search_path, &mut limitations);
        let mut layers = Vec::new();
        for layer in input.layers {
            let mut layer_limitations = Vec::new();
            let canonical_layer = canonical_directory(&layer.identity.root).ok();
            let roots = validate_report_roots(&layer.report_roots, &mut layer_limitations);
            let run = match canonical_layer {
                None => QaLayerRunCapability::Disabled("configured layer root is unsafe".into()),
                Some(root) if root != layer.identity.root => {
                    QaLayerRunCapability::Disabled("configured layer root is not canonical".into())
                }
                Some(root) => match &executable {
                    Some(executable) => QaLayerRunCapability::Available {
                        executable: executable.clone(),
                        arguments: vec![root.display().to_string()],
                        report_roots: roots,
                    },
                    None => QaLayerRunCapability::Disabled(
                        "yocto-check-layer was not found as a canonical executable".into(),
                    ),
                },
            };
            if let Some(reason) = run.disabled_reason() {
                layer_limitations.push(reason.into());
            }
            let capability = QaConfiguredLayerCapability::new(
                layer.check,
                layer.identity,
                layer.compatible_series,
                run,
                layer_limitations.clone(),
            )
            .map_err(|message| QaLayerAdapterError::InvalidInput(message.into()))?;
            limitations.extend(layer_limitations);
            layers.push(capability);
        }
        let snapshot = QaLayerCapabilitySnapshot::new(
            input.release,
            build_directory,
            input.selected_layer,
            layers,
            limitations.clone(),
        )
        .map_err(|message| QaLayerAdapterError::InvalidInput(message.into()))?;
        if limitations.is_empty() {
            Ok(QaLayerCapabilityResponse::Available(snapshot))
        } else {
            Ok(QaLayerCapabilityResponse::Partial(snapshot))
        }
    }
}

fn discover_executable(
    search_path: &[PathBuf],
    limitations: &mut Vec<String>,
) -> Option<QaExecutableIdentity> {
    for directory in search_path {
        let Ok(canonical) = canonical_directory(directory) else {
            limitations.push(format!(
                "ignored unsafe layer-QA executable search directory: {}",
                directory.display()
            ));
            continue;
        };
        if canonical != *directory {
            limitations.push(format!(
                "ignored non-canonical layer-QA executable search directory: {}",
                directory.display()
            ));
            continue;
        }
        let candidate = directory.join("yocto-check-layer");
        if !candidate.exists() {
            continue;
        }
        match executable_identity(&candidate) {
            Ok(identity) => return Some(identity),
            Err(_) => limitations.push(format!(
                "ignored unsafe yocto-check-layer candidate: {}",
                candidate.display()
            )),
        }
    }
    None
}

fn validate_report_roots(paths: &[PathBuf], limitations: &mut Vec<String>) -> Vec<PathBuf> {
    if paths.len() > MAX_QA_REPORT_PATHS {
        limitations.push(format!(
            "report roots exceeded the {MAX_QA_REPORT_PATHS}-path bound"
        ));
    }
    let mut roots = Vec::new();
    for path in paths.iter().take(MAX_QA_REPORT_PATHS) {
        match canonical_file_or_directory(path) {
            Ok(canonical) if canonical == *path => roots.push(canonical),
            _ => limitations.push(format!("ignored unsafe QA report root: {}", path.display())),
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

fn executable_identity(path: &Path) -> Result<QaExecutableIdentity, QaLayerAdapterError> {
    if path.file_name() != Some(OsStr::new("yocto-check-layer")) {
        return Err(QaLayerAdapterError::UnsafeExecutable(path.into()));
    }
    let metadata = safe_metadata(path, false)
        .map_err(|_| QaLayerAdapterError::UnsafeExecutable(path.into()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(QaLayerAdapterError::UnsafeExecutable(path.into()));
        }
    }
    let canonical =
        fs::canonicalize(path).map_err(|_| QaLayerAdapterError::UnsafeExecutable(path.into()))?;
    if canonical != path {
        return Err(QaLayerAdapterError::UnsafeExecutable(path.into()));
    }
    QaExecutableIdentity::new(
        canonical,
        metadata.len(),
        metadata
            .modified()
            .map_err(|_| QaLayerAdapterError::UnsafeExecutable(path.into()))?,
    )
    .map_err(|_| QaLayerAdapterError::UnsafeExecutable(path.into()))
}

fn revalidate_executable(identity: &QaExecutableIdentity) -> Result<(), QaLayerAdapterError> {
    let current = executable_identity(&identity.path)?;
    if &current != identity {
        return Err(QaLayerAdapterError::StaleIdentity(identity.path.clone()));
    }
    Ok(())
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

fn canonical_directory(path: &Path) -> Result<PathBuf, ()> {
    let metadata = safe_metadata(path, true)?;
    if !metadata.is_dir() {
        return Err(());
    }
    let canonical = fs::canonicalize(path).map_err(|_| ())?;
    (canonical == path).then_some(canonical).ok_or(())
}

fn canonical_file_or_directory(path: &Path) -> Result<PathBuf, ()> {
    safe_metadata(path, true)?;
    let canonical = fs::canonicalize(path).map_err(|_| ())?;
    (canonical == path).then_some(canonical).ok_or(())
}
