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
    CveFinding, CveFindingIdentity, CveReport, CveStatus, CycloneDxDocument,
    MAX_SECURITY_COMPONENTS, MAX_SECURITY_FINDINGS, MAX_SECURITY_LIMITATIONS,
    MAX_SECURITY_METADATA, MAX_SECURITY_REPORTS, MAX_SECURITY_TEXT_BYTES, PackageManifestDocument,
    SecurityMetadata, SecurityReport, SecurityReportIdentity, SecurityReportRequest,
    SpdxArtifactKind, SpdxComponent, SpdxDocument, normalize_security_reports,
};

const MAX_SECURITY_DIRECTORIES: usize = 128;
const MAX_SECURITY_DIRECTORY_ENTRIES: usize = 4_096;
const MAX_SECURITY_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SECURITY_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
const SECURITY_REPORT_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PARSE_DEPTH: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityReportScanOutcome {
    Empty,
    Complete(Vec<SecurityReport>),
    Partial {
        reports: Vec<SecurityReport>,
        limitations: Vec<String>,
    },
}

impl SecurityReportScanOutcome {
    pub fn reports(&self) -> &[SecurityReport] {
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
pub struct SecurityReportResponse {
    pub request: SecurityReportRequest,
    pub outcome: SecurityReportScanOutcome,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SecurityReportAdapterError {
    #[error("invalid Security report request: {0}")]
    InvalidRequest(String),
    #[error("Security report path is missing: {0}")]
    MissingPath(PathBuf),
    #[error("Security report path is not an absolute canonical regular file or directory: {0}")]
    UnsafePath(PathBuf),
    #[error("Security report path must not be a symlink: {0}")]
    SymlinkPath(PathBuf),
    #[error("Security report path escaped its explicit root or was not canonical: {0}")]
    EscapePath(PathBuf),
    #[error("Security report format is unsupported: {0}")]
    UnsupportedPath(PathBuf),
    #[error("Security report was malformed: {0}")]
    MalformedReport(PathBuf),
    #[error("Security report exceeded the bounded size: {0}")]
    OversizedReport(PathBuf),
    #[error("Security report changed during acquisition: {0}")]
    StaleReport(PathBuf),
    #[error("Security report path permission was denied: {0}")]
    PermissionDenied(PathBuf),
    #[error("Security report scan timed out after {0} seconds")]
    Timeout(u64),
    #[error("Security report scan was cancelled")]
    Cancelled,
    #[error("Security report scan worker was lost: {0}")]
    WorkerLost(String),
    #[error("Security report acquisition failed: {0}")]
    Io(String),
    #[error("no usable Security reports were found: {0}")]
    NoUsableReports(String),
}

#[derive(Debug, Clone, Default)]
pub struct SecurityReportCancellation {
    requested: Arc<AtomicBool>,
}

#[derive(Debug, Default)]
struct ScanFailures {
    symlink: Option<PathBuf>,
    escape: Option<PathBuf>,
    unsupported: Option<PathBuf>,
    malformed: Option<PathBuf>,
    oversized: Option<PathBuf>,
    stale: Option<PathBuf>,
}

impl ScanFailures {
    fn terminal_error(self, limitations: &[String]) -> SecurityReportAdapterError {
        if let Some(path) = self.stale {
            SecurityReportAdapterError::StaleReport(path)
        } else if let Some(path) = self.symlink {
            SecurityReportAdapterError::SymlinkPath(path)
        } else if let Some(path) = self.escape {
            SecurityReportAdapterError::EscapePath(path)
        } else if let Some(path) = self.oversized {
            SecurityReportAdapterError::OversizedReport(path)
        } else if let Some(path) = self.malformed {
            SecurityReportAdapterError::MalformedReport(path)
        } else if let Some(path) = self.unsupported {
            SecurityReportAdapterError::UnsupportedPath(path)
        } else {
            SecurityReportAdapterError::NoUsableReports(limitations.join("; "))
        }
    }
}

impl SecurityReportCancellation {
    pub fn cancel(&self) -> bool {
        !self.requested.swap(true, Ordering::SeqCst)
    }

    pub fn is_cancelled(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone)]
pub struct SecurityReportAdapter {
    timeout: Duration,
    #[cfg(test)]
    panic_worker: bool,
}

impl Default for SecurityReportAdapter {
    fn default() -> Self {
        Self {
            timeout: SECURITY_REPORT_TIMEOUT,
            #[cfg(test)]
            panic_worker: false,
        }
    }
}

impl SecurityReportAdapter {
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
        request: SecurityReportRequest,
    ) -> Result<SecurityReportResponse, SecurityReportAdapterError> {
        self.scan_with_cancellation(request, SecurityReportCancellation::default())
            .await
    }

    pub async fn scan_with_cancellation(
        &self,
        request: SecurityReportRequest,
        cancellation: SecurityReportCancellation,
    ) -> Result<SecurityReportResponse, SecurityReportAdapterError> {
        validate_request(&request)?;
        if cancellation.is_cancelled() {
            return Err(SecurityReportAdapterError::Cancelled);
        }
        if self.timeout.is_zero() {
            return Err(SecurityReportAdapterError::Timeout(0));
        }

        let deadline = Instant::now() + self.timeout;
        #[cfg(test)]
        let panic_worker = self.panic_worker;
        let task = tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            if panic_worker {
                panic!("synthetic Security report worker loss");
            }
            scan_reports(request, cancellation, deadline)
        });
        match tokio::time::timeout(self.timeout, task).await {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => Err(SecurityReportAdapterError::WorkerLost(error.to_string())),
            Err(_) => Err(SecurityReportAdapterError::Timeout(self.timeout.as_secs())),
        }
    }
}
