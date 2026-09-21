use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, UNIX_EPOCH},
};

use thiserror::Error;
use yoctui_model::{
    MAX_SDK_ARTIFACTS, MAX_SDK_ASSOCIATIONS, SdkArtifact, SdkArtifactIdentity,
    SdkArtifactInventoryRequest, SdkArtifactKind, normalize_sdk_artifacts,
    normalize_sdk_limitations,
};

const MAX_SDK_DIRECTORIES: usize = 128;
const MAX_DIRECTORY_ENTRIES: usize = 4_096;
const MAX_SDK_PATH_BYTES: usize = 4_096;
const MAX_SDK_NAME_BYTES: usize = 240;
const SDK_ARTIFACT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdkArtifactScanOutcome {
    Empty,
    Complete(Vec<SdkArtifact>),
    Partial {
        artifacts: Vec<SdkArtifact>,
        limitations: Vec<String>,
    },
}

impl SdkArtifactScanOutcome {
    pub fn artifacts(&self) -> &[SdkArtifact] {
        match self {
            Self::Empty => &[],
            Self::Complete(artifacts) | Self::Partial { artifacts, .. } => artifacts,
        }
    }

    pub fn limitations(&self) -> &[String] {
        match self {
            Self::Partial { limitations, .. } => limitations,
            Self::Empty | Self::Complete(_) => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkArtifactResponse {
    pub request: SdkArtifactInventoryRequest,
    pub outcome: SdkArtifactScanOutcome,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SdkArtifactAdapterError {
    #[error("invalid SDK artifact request: {0}")]
    InvalidRequest(String),
    #[error(
        "configured SDK deploy root does not match the request: configured {configured}, requested {requested}"
    )]
    RootMismatch {
        configured: PathBuf,
        requested: PathBuf,
    },
    #[error("SDK deploy root does not exist: {0}")]
    MissingRoot(PathBuf),
    #[error("SDK deploy root permission was denied: {0}")]
    PermissionDenied(PathBuf),
    #[error("SDK deploy root must be an absolute canonical directory: {0}")]
    InvalidRoot(PathBuf),
    #[error("SDK deploy root must not be a symlink: {0}")]
    SymlinkRoot(PathBuf),
    #[error("SDK artifact scan timed out after {0} seconds")]
    Timeout(u64),
    #[error("SDK artifact scan was cancelled")]
    Cancelled,
    #[error("SDK artifact scan worker was lost: {0}")]
    WorkerLost(String),
    #[error("SDK artifact I/O failed: {0}")]
    Io(String),
}

#[derive(Debug, Clone, Default)]
pub struct SdkArtifactCancellation {
    requested: Arc<AtomicBool>,
}

impl SdkArtifactCancellation {
    pub fn cancel(&self) -> bool {
        !self.requested.swap(true, Ordering::SeqCst)
    }

    pub fn is_cancelled(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone)]
pub struct SdkArtifactAdapter {
    deploy_root: PathBuf,
    timeout: Duration,
    #[cfg(test)]
    panic_worker: bool,
}

impl SdkArtifactAdapter {
    pub fn new(deploy_root: PathBuf) -> Self {
        Self {
            deploy_root,
            timeout: SDK_ARTIFACT_TIMEOUT,
            #[cfg(test)]
            panic_worker: false,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    #[cfg(test)]
    fn with_worker_panic(mut self) -> Self {
        self.panic_worker = true;
        self
    }

    pub async fn scan(
        &self,
        request: SdkArtifactInventoryRequest,
    ) -> Result<SdkArtifactResponse, SdkArtifactAdapterError> {
        self.scan_with_cancellation(request, SdkArtifactCancellation::default())
            .await
    }

    pub async fn scan_with_cancellation(
        &self,
        request: SdkArtifactInventoryRequest,
        cancellation: SdkArtifactCancellation,
    ) -> Result<SdkArtifactResponse, SdkArtifactAdapterError> {
        request
            .validate()
            .map_err(|message| SdkArtifactAdapterError::InvalidRequest(message.into()))?;
        if request.root != self.deploy_root {
            return Err(SdkArtifactAdapterError::RootMismatch {
                configured: self.deploy_root.clone(),
                requested: request.root,
            });
        }
        if cancellation.is_cancelled() {
            return Err(SdkArtifactAdapterError::Cancelled);
        }

        let deploy_root = self.deploy_root.clone();
        let deadline = Instant::now() + self.timeout;
        #[cfg(test)]
        let panic_worker = self.panic_worker;
        let task = tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            if panic_worker {
                panic!("synthetic SDK scan worker loss");
            }
            scan_deploy_root(request, deploy_root, cancellation, deadline)
        });
        match tokio::time::timeout(self.timeout, task).await {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => Err(SdkArtifactAdapterError::WorkerLost(error.to_string())),
            Err(_) => Err(SdkArtifactAdapterError::Timeout(self.timeout.as_secs())),
        }
    }
}

#[derive(Debug)]
struct FileRecord {
    path: PathBuf,
    size_bytes: u64,
    modified_unix_seconds: u64,
    kind: SdkArtifactKind,
}
