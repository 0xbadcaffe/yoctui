use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use yoctui_model::{
    MAX_QA_FINDINGS, MAX_QA_LIMITATIONS, MAX_QA_METADATA, MAX_QA_REPORTS, MAX_QA_TEXT_BYTES,
    QaCheckId, QaFinding, QaFindingIdentity, QaFindingScope, QaFindingStatus, QaMetadata, QaReport,
    QaReportFormat, QaReportIdentity, QaReportRequest, QaSourceLocation, normalize_qa_reports,
};

const MAX_QA_DIRECTORIES: usize = 128;
const MAX_QA_DIRECTORY_ENTRIES: usize = 4_096;
const MAX_QA_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_QA_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
const QA_REPORT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QaReportOrigin {
    Managed,
    Import,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaReportCandidate {
    pub path: PathBuf,
    pub origin: QaReportOrigin,
    pub format: Option<QaReportFormat>,
    pub producer: QaCheckId,
    pub scope: QaFindingScope,
    pub task: Option<String>,
    pub test_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaReportScanInput {
    pub build_directory: PathBuf,
    pub request: QaReportRequest,
    pub candidates: Vec<QaReportCandidate>,
    pub known_checks: Vec<QaCheckId>,
    pub known_scopes: Vec<QaFindingScope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaReportScanOutcome {
    Empty,
    Complete(Vec<QaReport>),
    Partial {
        reports: Vec<QaReport>,
        limitations: Vec<String>,
    },
}

impl QaReportScanOutcome {
    pub fn reports(&self) -> &[QaReport] {
        match self {
            Self::Empty => &[],
            Self::Complete(reports) | Self::Partial { reports, .. } => reports,
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
pub struct QaReportResponse {
    pub request: QaReportRequest,
    pub outcome: QaReportScanOutcome,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum QaReportAdapterError {
    #[error("invalid QA report request: {0}")]
    InvalidRequest(String),
    #[error("QA report path is missing: {0}")]
    MissingPath(PathBuf),
    #[error("QA report path permission was denied: {0}")]
    PermissionDenied(PathBuf),
    #[error("QA report path is unsafe: {0}")]
    UnsafePath(PathBuf),
    #[error("QA report path must not be a symlink: {0}")]
    SymlinkPath(PathBuf),
    #[error("QA report path escaped its exact root: {0}")]
    EscapePath(PathBuf),
    #[error("QA report format is unsupported: {0}")]
    UnsupportedPath(PathBuf),
    #[error("QA report was malformed: {0}")]
    MalformedReport(PathBuf),
    #[error("QA report exceeded a hard bound: {0}")]
    OversizedReport(PathBuf),
    #[error("QA report identity is stale: {0}")]
    StaleReport(PathBuf),
    #[error("QA report acquisition timed out after {0} seconds")]
    Timeout(u64),
    #[error("QA report acquisition was cancelled")]
    Cancelled,
    #[error("QA report worker was lost: {0}")]
    WorkerLost(String),
    #[error("QA report acquisition failed: {0}")]
    Io(String),
    #[error("no usable QA reports were found: {0}")]
    NoUsableReports(String),
}

#[derive(Debug, Clone, Default)]
pub struct QaReportCancellation {
    requested: Arc<AtomicBool>,
}

impl QaReportCancellation {
    pub fn cancel(&self) -> bool {
        !self.requested.swap(true, Ordering::SeqCst)
    }

    pub fn is_cancelled(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone)]
pub struct QaReportAdapter {
    timeout: Duration,
    #[cfg(test)]
    panic_worker: bool,
}

impl Default for QaReportAdapter {
    fn default() -> Self {
        Self {
            timeout: QA_REPORT_TIMEOUT,
            #[cfg(test)]
            panic_worker: false,
        }
    }
}

impl QaReportAdapter {
    pub fn new() -> Self {
        Self::default()
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
        input: QaReportScanInput,
    ) -> Result<QaReportResponse, QaReportAdapterError> {
        self.scan_with_cancellation(input, QaReportCancellation::default())
            .await
    }

    pub async fn scan_with_cancellation(
        &self,
        input: QaReportScanInput,
        cancellation: QaReportCancellation,
    ) -> Result<QaReportResponse, QaReportAdapterError> {
        validate_input(&input)?;
        if cancellation.is_cancelled() {
            return Err(QaReportAdapterError::Cancelled);
        }
        if self.timeout.is_zero() {
            return Err(QaReportAdapterError::Timeout(0));
        }
        let deadline = Instant::now() + self.timeout;
        #[cfg(test)]
        let panic_worker = self.panic_worker;
        let task = tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            if panic_worker {
                panic!("synthetic QA report worker loss");
            }
            scan_reports(input, cancellation, deadline)
        });
        match tokio::time::timeout(self.timeout, task).await {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => Err(QaReportAdapterError::WorkerLost(error.to_string())),
            Err(_) => Err(QaReportAdapterError::Timeout(self.timeout.as_secs())),
        }
    }

    pub fn revalidate(&self, identity: &QaReportIdentity) -> Result<(), QaReportAdapterError> {
        let before = validate_regular_file(&identity.path)?;
        if before.len() != identity.byte_size
            || before.modified().ok() != Some(identity.modified_at)
        {
            return Err(QaReportAdapterError::StaleReport(identity.path.clone()));
        }
        let bytes = read_bounded_file(
            &identity.path,
            identity.byte_size,
            &QaReportCancellation::default(),
            Instant::now() + self.timeout,
        )?;
        let after = validate_regular_file(&identity.path)?;
        if after.len() != identity.byte_size
            || after.modified().ok() != Some(identity.modified_at)
            || fingerprint(&bytes) != identity.fingerprint
        {
            return Err(QaReportAdapterError::StaleReport(identity.path.clone()));
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
struct ScanFailures {
    unsupported: Option<PathBuf>,
    malformed: Option<PathBuf>,
    oversized: Option<PathBuf>,
    stale: Option<PathBuf>,
    unsafe_path: Option<PathBuf>,
}

impl ScanFailures {
    fn terminal_error(self, limitations: &[String]) -> QaReportAdapterError {
        if let Some(path) = self.stale {
            QaReportAdapterError::StaleReport(path)
        } else if let Some(path) = self.unsafe_path {
            QaReportAdapterError::UnsafePath(path)
        } else if let Some(path) = self.oversized {
            QaReportAdapterError::OversizedReport(path)
        } else if let Some(path) = self.malformed {
            QaReportAdapterError::MalformedReport(path)
        } else if let Some(path) = self.unsupported {
            QaReportAdapterError::UnsupportedPath(path)
        } else {
            QaReportAdapterError::NoUsableReports(limitations.join("; "))
        }
    }
}

#[derive(Debug, Clone)]
struct ExactFile {
    path: PathBuf,
    format: QaReportFormat,
    producer: QaCheckId,
    scope: QaFindingScope,
    task: Option<String>,
    test_name: Option<String>,
}
