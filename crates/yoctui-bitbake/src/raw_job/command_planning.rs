use std::{
    ffi::OsString,
    io,
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
    CapabilityToolId, DaemonCompatibilitySnapshot, RawAdditionalArguments,
    RawConfirmedExecutionRequest, RawInteractionMode, RawJobId, RawOutputChunk, RawOutputStream,
    RawPreviewRequest, RawRequestId, RawSessionId, RawStreamId, builtin_raw_catalog,
};

use crate::output_text;

const RAW_JOB_EVENT_CHANNEL_CAPACITY: usize = 256;
const RAW_JOB_DEFAULT_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);
const RAW_JOB_DEFAULT_CANCELLATION_TIMEOUT: Duration = Duration::from_secs(5);
const RAW_JOB_SPAWN_ATTEMPTS: usize = 4;
const RAW_JOB_SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawJobCommandSpec {
    request_id: RawRequestId,
    job_id: RawJobId,
    stdout_stream: RawStreamId,
    stderr_stream: RawStreamId,
    executable: PathBuf,
    arguments: Vec<OsString>,
    current_directory: PathBuf,
    capability_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPtyCommandSpec {
    request_id: RawRequestId,
    session_id: RawSessionId,
    executable: PathBuf,
    arguments: Vec<String>,
    current_directory: PathBuf,
    capability_generation: u64,
}

impl RawPtyCommandSpec {
    pub fn request_id(&self) -> &RawRequestId {
        &self.request_id
    }

    pub fn session_id(&self) -> &RawSessionId {
        &self.session_id
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn current_directory(&self) -> &Path {
        &self.current_directory
    }

    pub const fn capability_generation(&self) -> u64 {
        self.capability_generation
    }
}

impl RawJobCommandSpec {
    pub fn request_id(&self) -> &RawRequestId {
        &self.request_id
    }

    pub fn job_id(&self) -> &RawJobId {
        &self.job_id
    }

    pub fn stdout_stream(&self) -> &RawStreamId {
        &self.stdout_stream
    }

    pub fn stderr_stream(&self) -> &RawStreamId {
        &self.stderr_stream
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn current_directory(&self) -> &Path {
        &self.current_directory
    }

    pub const fn capability_generation(&self) -> u64 {
        self.capability_generation
    }
}

pub struct RawJobPlanner<'a> {
    compatibility: &'a DaemonCompatibilitySnapshot,
}

pub struct RawPtyPlanner<'a> {
    compatibility: &'a DaemonCompatibilitySnapshot,
}

impl<'a> RawPtyPlanner<'a> {
    pub fn new(compatibility: &'a DaemonCompatibilitySnapshot) -> Self {
        Self { compatibility }
    }

    pub fn plan(
        &self,
        request: &RawConfirmedExecutionRequest,
        session_id: RawSessionId,
    ) -> Result<RawPtyCommandSpec, RawJobPlannerError> {
        if request.interaction != RawInteractionMode::InteractivePty {
            return Err(RawJobPlannerError::NoninteractiveRequest);
        }
        let (preview, executable) = reconstruct_raw_command(self.compatibility, request)?;
        Ok(RawPtyCommandSpec {
            request_id: request.id.clone(),
            session_id,
            executable,
            arguments: preview.arguments,
            current_directory: request.build_directory.clone(),
            capability_generation: request.capability_generation,
        })
    }
}

impl<'a> RawJobPlanner<'a> {
    pub fn new(compatibility: &'a DaemonCompatibilitySnapshot) -> Self {
        Self { compatibility }
    }

    pub fn plan(
        &self,
        request: &RawConfirmedExecutionRequest,
        job_id: RawJobId,
        stdout_stream: RawStreamId,
        stderr_stream: RawStreamId,
    ) -> Result<RawJobCommandSpec, RawJobPlannerError> {
        if request.interaction != RawInteractionMode::NoninteractiveJob {
            return Err(RawJobPlannerError::InteractiveRequest);
        }
        if stdout_stream == stderr_stream {
            return Err(RawJobPlannerError::DuplicateStreamIdentity);
        }
        let (preview, executable) = reconstruct_raw_command(self.compatibility, request)?;
        Ok(RawJobCommandSpec {
            request_id: request.id.clone(),
            job_id,
            stdout_stream,
            stderr_stream,
            executable,
            arguments: preview.arguments.iter().map(OsString::from).collect(),
            current_directory: request.build_directory.clone(),
            capability_generation: request.capability_generation,
        })
    }
}

fn reconstruct_raw_command(
    compatibility: &DaemonCompatibilitySnapshot,
    request: &RawConfirmedExecutionRequest,
) -> Result<(yoctui_model::RawExecutionPreview, PathBuf), RawJobPlannerError> {
    request
        .validate()
        .map_err(|error| RawJobPlannerError::InvalidRequest(error.to_string()))?;
    let additional_arguments =
        RawAdditionalArguments::from_vec(request.additional_arguments.clone())
            .map_err(|error| RawJobPlannerError::InvalidRequest(error.to_string()))?;
    let preview_request = RawPreviewRequest {
        catalog_version: request.catalog_version,
        command: request.command.clone(),
        parameters: request.parameters.clone(),
        additional_arguments,
        capability_generation: request.capability_generation,
        build_directory: request.build_directory.clone(),
    };
    let catalog = builtin_raw_catalog();
    let preview = catalog
        .preview(&preview_request, Some(compatibility))
        .map_err(|error| RawJobPlannerError::Authorization(error.to_string()))?;
    let reconstructed = RawConfirmedExecutionRequest::from_reviewed_preview(
        request.id.clone(),
        catalog,
        &preview_request,
        &preview,
    )
    .map_err(|error| RawJobPlannerError::Authorization(error.to_string()))?;
    if reconstructed != *request {
        return Err(RawJobPlannerError::PreviewMismatch);
    }
    let executable = compatibility
        .snapshot
        .environment
        .available_tools
        .value()
        .and_then(|tools| {
            tools
                .iter()
                .find(|tool| tool.id == CapabilityToolId::BitBake.executable_name())
        })
        .map(|tool| tool.executable.clone())
        .ok_or(RawJobPlannerError::MissingExecutableAuthority)?;
    validate_directory(&request.build_directory)?;
    validate_executable(&executable)?;
    Ok((preview, executable))
}

fn validate_directory(path: &Path) -> Result<(), RawJobPlannerError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| RawJobPlannerError::UnsafeBuildDirectory(path.into(), error.kind()))?;
    let canonical = std::fs::canonicalize(path)
        .map_err(|error| RawJobPlannerError::UnsafeBuildDirectory(path.into(), error.kind()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() || canonical != path {
        return Err(RawJobPlannerError::UnsafeBuildDirectory(
            path.into(),
            io::ErrorKind::InvalidInput,
        ));
    }
    Ok(())
}

fn validate_executable(path: &Path) -> Result<(), RawJobPlannerError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| RawJobPlannerError::UnsafeExecutable(path.into(), error.kind()))?;
    let canonical = std::fs::canonicalize(path)
        .map_err(|error| RawJobPlannerError::UnsafeExecutable(path.into(), error.kind()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || canonical != path {
        return Err(RawJobPlannerError::UnsafeExecutable(
            path.into(),
            io::ErrorKind::InvalidInput,
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(RawJobPlannerError::UnsafeExecutable(
                path.into(),
                io::ErrorKind::PermissionDenied,
            ));
        }
    }
    Ok(())
}
