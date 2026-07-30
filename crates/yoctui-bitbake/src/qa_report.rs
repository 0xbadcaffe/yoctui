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

fn validate_input(input: &QaReportScanInput) -> Result<(), QaReportAdapterError> {
    let normalized = QaReportRequest::new(input.request.generation, input.request.paths.clone())
        .map_err(|message| QaReportAdapterError::InvalidRequest(message.into()))?;
    if normalized != input.request {
        return Err(QaReportAdapterError::InvalidRequest(
            "request paths must be sorted and unique".into(),
        ));
    }
    let build = validate_directory(&input.build_directory)?;
    if build != input.build_directory {
        return Err(QaReportAdapterError::InvalidRequest(
            "build directory must be canonical".into(),
        ));
    }
    if input.candidates.is_empty() || input.candidates.len() > input.request.paths.len() {
        return Err(QaReportAdapterError::InvalidRequest(
            "candidate count does not match the exact request".into(),
        ));
    }
    let mut candidate_paths = input
        .candidates
        .iter()
        .map(|candidate| candidate.path.clone())
        .collect::<Vec<_>>();
    candidate_paths.sort();
    if candidate_paths != input.request.paths {
        return Err(QaReportAdapterError::InvalidRequest(
            "candidate paths do not match the exact request".into(),
        ));
    }
    let mut unique = BTreeSet::new();
    for candidate in &input.candidates {
        if !unique.insert(candidate.path.clone())
            || !input.known_checks.contains(&candidate.producer)
            || !input.known_scopes.contains(&candidate.scope)
            || !candidate.producer.is_valid()
            || !candidate.scope.is_valid()
            || candidate
                .task
                .as_deref()
                .is_some_and(|value| !bounded_token(value))
            || candidate
                .test_name
                .as_deref()
                .is_some_and(|value| !bounded_text(value))
            || !matches!(
                (&candidate.scope, &candidate.task, &candidate.test_name),
                (QaFindingScope::Recipe(_), _, None) | (QaFindingScope::Layer(_), None, Some(_))
            )
        {
            return Err(QaReportAdapterError::InvalidRequest(
                "candidate scope, producer, task, or identity is invalid".into(),
            ));
        }
    }
    Ok(())
}

fn scan_reports(
    input: QaReportScanInput,
    cancellation: QaReportCancellation,
    deadline: Instant,
) -> Result<QaReportResponse, QaReportAdapterError> {
    let build_directory = validate_directory(&input.build_directory)?;
    let mut files = BTreeMap::<PathBuf, ExactFile>::new();
    let mut limitations = Vec::new();
    let mut failures = ScanFailures::default();
    let mut directory_count = 0_usize;

    for candidate in &input.candidates {
        check_control(&cancellation, deadline)?;
        let root = validate_explicit_path(&candidate.path)?;
        if candidate.origin == QaReportOrigin::Managed && !root.starts_with(&build_directory) {
            return Err(QaReportAdapterError::EscapePath(root));
        }
        let metadata = fs::symlink_metadata(&root).map_err(|error| path_error(&root, error))?;
        if metadata.is_file() {
            let Some(format) = candidate.format else {
                return Err(QaReportAdapterError::InvalidRequest(
                    "an exact file candidate must supply its documented format".into(),
                ));
            };
            if documented_format(&root) != Some(format) {
                return Err(QaReportAdapterError::UnsupportedPath(root));
            }
            files.insert(root.clone(), exact_file(candidate, root, format));
        } else {
            if candidate.format.is_some() {
                return Err(QaReportAdapterError::InvalidRequest(
                    "a directory candidate must not force one file format".into(),
                ));
            }
            collect_directory_files(
                &root,
                &root,
                candidate,
                &mut files,
                &mut directory_count,
                &mut limitations,
                &mut failures,
                &cancellation,
                deadline,
            )?;
        }
    }

    let mut reports = Vec::new();
    let mut total_bytes = 0_u64;
    for file in files.into_values() {
        check_control(&cancellation, deadline)?;
        if reports.len() >= MAX_QA_REPORTS {
            push_limitation(
                &mut limitations,
                format!(
                    "QA report {} was omitted at the {MAX_QA_REPORTS}-report bound",
                    file.path.display()
                ),
            );
            continue;
        }
        let before = validate_regular_file(&file.path)?;
        let size = before.len();
        if size == 0 {
            failures.malformed.get_or_insert_with(|| file.path.clone());
            push_limitation(
                &mut limitations,
                format!("empty QA report was ignored: {}", file.path.display()),
            );
            continue;
        }
        if size > MAX_QA_FILE_BYTES || total_bytes.saturating_add(size) > MAX_QA_TOTAL_BYTES {
            failures.oversized.get_or_insert_with(|| file.path.clone());
            push_limitation(
                &mut limitations,
                format!("oversized QA report was ignored: {}", file.path.display()),
            );
            continue;
        }
        let Some(modified_at) = before.modified().ok() else {
            failures
                .unsafe_path
                .get_or_insert_with(|| file.path.clone());
            push_limitation(
                &mut limitations,
                format!(
                    "QA report modification time was unavailable: {}",
                    file.path.display()
                ),
            );
            continue;
        };
        let bytes = read_bounded_file(&file.path, size, &cancellation, deadline)?;
        total_bytes = total_bytes.saturating_add(bytes.len() as u64);
        let after = validate_regular_file(&file.path)?;
        if after.len() != size
            || after.modified().ok() != Some(modified_at)
            || fs::canonicalize(&file.path).ok().as_ref() != Some(&file.path)
        {
            failures.stale.get_or_insert_with(|| file.path.clone());
            push_limitation(
                &mut limitations,
                format!(
                    "QA report changed while it was acquired: {}",
                    file.path.display()
                ),
            );
            continue;
        }
        let identity = QaReportIdentity::new(
            file.path.clone(),
            size,
            modified_at,
            fingerprint(&bytes),
            file.format,
            Some(file.producer.clone()),
            Some(file.scope.clone()),
        )
        .map_err(|message| QaReportAdapterError::Io(message.into()))?;
        match parse_report(identity, &file, &bytes, &mut limitations) {
            Ok(Some(report)) => reports.push(report),
            Ok(None) | Err(QaReportAdapterError::MalformedReport(_)) => {
                failures.malformed.get_or_insert_with(|| file.path.clone());
                push_limitation(
                    &mut limitations,
                    format!("malformed QA report was ignored: {}", file.path.display()),
                );
            }
            Err(error) => return Err(error),
        }
    }

    let (reports, model_limitations) =
        normalize_qa_reports(reports, &input.known_checks, &input.known_scopes);
    for limitation in model_limitations {
        push_limitation(&mut limitations, limitation);
    }
    for report in &reports {
        for limitation in &report.limitations {
            push_limitation(
                &mut limitations,
                format!("{}: {limitation}", report.identity.path.display()),
            );
        }
    }
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_QA_LIMITATIONS);
    if reports.is_empty() && !limitations.is_empty() {
        return Err(failures.terminal_error(&limitations));
    }
    let outcome = if reports.is_empty() {
        QaReportScanOutcome::Empty
    } else if limitations.is_empty() {
        QaReportScanOutcome::Complete(reports)
    } else {
        QaReportScanOutcome::Partial {
            reports,
            limitations,
        }
    };
    Ok(QaReportResponse {
        request: input.request,
        outcome,
    })
}

fn exact_file(candidate: &QaReportCandidate, path: PathBuf, format: QaReportFormat) -> ExactFile {
    ExactFile {
        path,
        format,
        producer: candidate.producer.clone(),
        scope: candidate.scope.clone(),
        task: candidate.task.clone(),
        test_name: candidate.test_name.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_directory_files(
    root: &Path,
    directory: &Path,
    candidate: &QaReportCandidate,
    files: &mut BTreeMap<PathBuf, ExactFile>,
    directory_count: &mut usize,
    limitations: &mut Vec<String>,
    failures: &mut ScanFailures,
    cancellation: &QaReportCancellation,
    deadline: Instant,
) -> Result<(), QaReportAdapterError> {
    if *directory_count >= MAX_QA_DIRECTORIES {
        push_limitation(
            limitations,
            format!(
                "QA directory was omitted at the {MAX_QA_DIRECTORIES}-directory bound: {}",
                directory.display()
            ),
        );
        return Ok(());
    }
    *directory_count += 1;
    let reader = fs::read_dir(directory).map_err(|error| path_error(directory, error))?;
    let mut entries = BTreeSet::new();
    let mut omitted = 0_usize;
    for entry in reader {
        check_control(cancellation, deadline)?;
        match entry {
            Ok(entry) => {
                entries.insert(entry.path());
                if entries.len() > MAX_QA_DIRECTORY_ENTRIES {
                    entries.pop_last();
                    omitted += 1;
                }
            }
            Err(error) => push_limitation(
                limitations,
                format!("a QA directory entry was unreadable: {error}"),
            ),
        }
    }
    if omitted > 0 {
        push_limitation(
            limitations,
            format!(
                "{omitted} QA entries were omitted from {} at the {MAX_QA_DIRECTORY_ENTRIES}-entry bound",
                directory.display()
            ),
        );
    }
    for path in entries {
        check_control(cancellation, deadline)?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                push_limitation(
                    limitations,
                    format!(
                        "QA entry metadata was unavailable for {}: {error}",
                        path.display()
                    ),
                );
                continue;
            }
        };
        if metadata.file_type().is_symlink() {
            failures.unsafe_path.get_or_insert_with(|| path.clone());
            push_limitation(
                limitations,
                format!("QA symlink was not followed: {}", path.display()),
            );
            continue;
        }
        let canonical = match fs::canonicalize(&path) {
            Ok(canonical)
                if canonical == path && canonical.starts_with(root) && canonical != root =>
            {
                canonical
            }
            _ => {
                failures.unsafe_path.get_or_insert_with(|| path.clone());
                push_limitation(
                    limitations,
                    format!("QA entry escaped its exact root: {}", path.display()),
                );
                continue;
            }
        };
        if metadata.is_dir() {
            collect_directory_files(
                root,
                &canonical,
                candidate,
                files,
                directory_count,
                limitations,
                failures,
                cancellation,
                deadline,
            )?;
        } else if metadata.is_file() {
            if let Some(format) = documented_format(&canonical) {
                if files.len() >= MAX_QA_REPORTS {
                    push_limitation(
                        limitations,
                        format!(
                            "QA report was omitted at the {MAX_QA_REPORTS}-file bound: {}",
                            canonical.display()
                        ),
                    );
                } else {
                    files.insert(canonical.clone(), exact_file(candidate, canonical, format));
                }
            } else {
                failures
                    .unsupported
                    .get_or_insert_with(|| canonical.clone());
                push_limitation(
                    limitations,
                    format!("unsupported QA entry was ignored: {}", canonical.display()),
                );
            }
        } else {
            failures
                .unsafe_path
                .get_or_insert_with(|| canonical.clone());
            push_limitation(
                limitations,
                format!("non-regular QA entry was ignored: {}", canonical.display()),
            );
        }
    }
    Ok(())
}

fn validate_explicit_path(path: &Path) -> Result<PathBuf, QaReportAdapterError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| path_error(path, error))?;
    if metadata.file_type().is_symlink() {
        return Err(QaReportAdapterError::SymlinkPath(path.into()));
    }
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(QaReportAdapterError::UnsafePath(path.into()));
    }
    let canonical =
        fs::canonicalize(path).map_err(|error| QaReportAdapterError::Io(error.to_string()))?;
    if canonical != path || canonical == Path::new("/") {
        return Err(QaReportAdapterError::EscapePath(path.into()));
    }
    Ok(canonical)
}

fn validate_directory(path: &Path) -> Result<PathBuf, QaReportAdapterError> {
    let canonical = validate_explicit_path(path)?;
    if !fs::symlink_metadata(&canonical)
        .map_err(|error| path_error(&canonical, error))?
        .is_dir()
    {
        return Err(QaReportAdapterError::UnsafePath(path.into()));
    }
    Ok(canonical)
}

fn validate_regular_file(path: &Path) -> Result<fs::Metadata, QaReportAdapterError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| path_error(path, error))?;
    if metadata.file_type().is_symlink() {
        return Err(QaReportAdapterError::SymlinkPath(path.into()));
    }
    if !metadata.is_file() || fs::canonicalize(path).ok().as_deref() != Some(path) {
        return Err(QaReportAdapterError::UnsafePath(path.into()));
    }
    Ok(metadata)
}

fn path_error(path: &Path, error: io::Error) -> QaReportAdapterError {
    match error.kind() {
        io::ErrorKind::NotFound => QaReportAdapterError::MissingPath(path.into()),
        io::ErrorKind::PermissionDenied => QaReportAdapterError::PermissionDenied(path.into()),
        _ => QaReportAdapterError::Io(format!("{}: {error}", path.display())),
    }
}

fn documented_format(path: &Path) -> Option<QaReportFormat> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    if name.ends_with(".json") || name.ends_with(".jsonl") {
        Some(QaReportFormat::Json)
    } else if name.ends_with(".xml") {
        Some(QaReportFormat::Xml)
    } else if name.ends_with(".qa") || name.ends_with(".txt") {
        Some(QaReportFormat::Text)
    } else if name.ends_with(".log") {
        Some(QaReportFormat::BitBakeLog)
    } else {
        None
    }
}

fn read_bounded_file(
    path: &Path,
    expected_size: u64,
    cancellation: &QaReportCancellation,
    deadline: Instant,
) -> Result<Vec<u8>, QaReportAdapterError> {
    if expected_size > MAX_QA_FILE_BYTES {
        return Err(QaReportAdapterError::OversizedReport(path.into()));
    }
    let mut file = fs::File::open(path).map_err(|error| path_error(path, error))?;
    let mut bytes = Vec::with_capacity(expected_size as usize);
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_control(cancellation, deadline)?;
        let read = file
            .read(&mut buffer)
            .map_err(|error| QaReportAdapterError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read) > MAX_QA_FILE_BYTES as usize {
            return Err(QaReportAdapterError::OversizedReport(path.into()));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn fingerprint(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn check_control(
    cancellation: &QaReportCancellation,
    deadline: Instant,
) -> Result<(), QaReportAdapterError> {
    if cancellation.is_cancelled() {
        Err(QaReportAdapterError::Cancelled)
    } else if Instant::now() >= deadline {
        Err(QaReportAdapterError::Timeout(0))
    } else {
        Ok(())
    }
}

fn parse_report(
    identity: QaReportIdentity,
    file: &ExactFile,
    bytes: &[u8],
    outer_limitations: &mut Vec<String>,
) -> Result<Option<QaReport>, QaReportAdapterError> {
    let mut limitations = Vec::new();
    let mut metadata = Vec::new();
    let findings = match identity.format {
        QaReportFormat::Json => parse_json_findings(file, bytes, &mut metadata, &mut limitations)?,
        QaReportFormat::Xml => parse_xml_findings(file, bytes, &mut limitations)?,
        QaReportFormat::Text => parse_text_findings(file, bytes, false, &mut limitations)?,
        QaReportFormat::BitBakeLog => parse_text_findings(file, bytes, true, &mut limitations)?,
    };
    if findings.is_empty() && !limitations.is_empty() {
        for limitation in &limitations {
            push_limitation(outer_limitations, limitation.clone());
        }
        return Ok(None);
    }
    Ok(Some(QaReport {
        identity,
        findings,
        metadata,
        limitations,
    }))
}

fn parse_json_findings(
    file: &ExactFile,
    bytes: &[u8],
    metadata: &mut Vec<QaMetadata>,
    limitations: &mut Vec<String>,
) -> Result<Vec<QaFinding>, QaReportAdapterError> {
    let values = if file
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".jsonl"))
    {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| QaReportAdapterError::MalformedReport(file.path.clone()))?;
        let mut values = Vec::new();
        for (index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str(line) {
                Ok(value) => values.push(value),
                Err(error) => push_limitation(
                    limitations,
                    format!("JSONL record {} was malformed: {error}", index + 1),
                ),
            }
            if values.len() >= MAX_QA_FINDINGS {
                push_limitation(
                    limitations,
                    format!("JSONL input reached the {MAX_QA_FINDINGS}-record bound"),
                );
                break;
            }
        }
        values
    } else {
        let value: Value = serde_json::from_slice(bytes)
            .map_err(|_| QaReportAdapterError::MalformedReport(file.path.clone()))?;
        match value {
            Value::Array(values) => values,
            Value::Object(mut object) => {
                if let Some(Value::Object(report_metadata)) = object.remove("metadata") {
                    *metadata = scalar_metadata(&report_metadata, limitations);
                }
                match object.remove("findings") {
                    Some(Value::Array(values)) => values,
                    _ if object.contains_key("status") && object.contains_key("message") => {
                        vec![Value::Object(object)]
                    }
                    _ => {
                        push_limitation(
                            limitations,
                            "JSON report contained no documented findings array".into(),
                        );
                        Vec::new()
                    }
                }
            }
            _ => {
                push_limitation(
                    limitations,
                    "JSON report root was not an object or array".into(),
                );
                Vec::new()
            }
        }
    };
    let omitted = values.len().saturating_sub(MAX_QA_FINDINGS);
    let mut findings = Vec::new();
    for value in values.into_iter().take(MAX_QA_FINDINGS) {
        let Some(object) = value.as_object() else {
            push_limitation(limitations, "a non-object JSON finding was ignored".into());
            continue;
        };
        match finding_from_object(file, object, limitations) {
            Some(finding) => findings.push(finding),
            None => push_limitation(limitations, "a malformed JSON finding was ignored".into()),
        }
    }
    if omitted > 0 {
        push_limitation(
            limitations,
            format!("{omitted} JSON findings were omitted at the {MAX_QA_FINDINGS}-record bound"),
        );
    }
    Ok(findings)
}

fn finding_from_object(
    file: &ExactFile,
    object: &Map<String, Value>,
    limitations: &mut Vec<String>,
) -> Option<QaFinding> {
    if object
        .get("check")
        .and_then(Value::as_str)
        .is_some_and(|value| value != file.producer.0)
    {
        push_limitation(
            limitations,
            "a finding for a different check identity was ignored".into(),
        );
        return None;
    }
    let message = bounded_json_string(object, "message", limitations)?;
    let raw_status = bounded_json_string(object, "status", limitations)?;
    let status = finding_status(&raw_status);
    if status == QaFindingStatus::Unknown && !raw_status.eq_ignore_ascii_case("unknown") {
        push_limitation(
            limitations,
            format!("unrecognized QA status was preserved as unknown: {raw_status}"),
        );
    }
    let source = object
        .get("source")
        .and_then(|value| parse_source(value, limitations));
    let record = serde_json::to_vec(object).ok()?;
    let identity = QaFindingIdentity::new(file.producer.clone(), fingerprint(&record)).ok()?;
    let mut metadata = object
        .get("metadata")
        .and_then(Value::as_object)
        .map(|values| scalar_metadata(values, limitations))
        .unwrap_or_default();
    metadata.sort();
    metadata.dedup();
    Some(QaFinding {
        identity,
        status,
        severity: bounded_json_string(object, "severity", limitations),
        message,
        scope: file.scope.clone(),
        task: file.task.clone(),
        test_name: bounded_json_string(object, "test_name", limitations)
            .or_else(|| file.test_name.clone()),
        source,
        rule: bounded_json_string(object, "rule", limitations),
        suggestion: bounded_json_string(object, "suggestion", limitations),
        metadata,
    })
    .filter(QaFinding::is_valid)
}

fn parse_source(value: &Value, limitations: &mut Vec<String>) -> Option<QaSourceLocation> {
    let object = value.as_object()?;
    let path = object.get("path")?.as_str().map(PathBuf::from)?;
    let line = object
        .get("line")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let column = object
        .get("column")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    if validate_regular_file(&path).is_err() {
        push_limitation(
            limitations,
            format!(
                "unsafe or missing QA source was ignored: {}",
                path.display()
            ),
        );
        return None;
    }
    QaSourceLocation::new(path, line, column).ok()
}

fn parse_text_findings(
    file: &ExactFile,
    bytes: &[u8],
    bitbake_log: bool,
    limitations: &mut Vec<String>,
) -> Result<Vec<QaFinding>, QaReportAdapterError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| QaReportAdapterError::MalformedReport(file.path.clone()))?;
    let mut findings = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parsed = if bitbake_log {
            parse_bitbake_line(file, line)
        } else {
            parse_tab_line(file, line)
        };
        match parsed {
            Some(finding) => findings.push(finding),
            None => push_limitation(
                limitations,
                format!("unsupported QA text record {} was ignored", index + 1),
            ),
        }
        if findings.len() >= MAX_QA_FINDINGS {
            push_limitation(
                limitations,
                format!("QA text reached the {MAX_QA_FINDINGS}-record bound"),
            );
            break;
        }
    }
    Ok(findings)
}

fn parse_tab_line(file: &ExactFile, line: &str) -> Option<QaFinding> {
    let mut fields = line.split('\t');
    let status_text = fields.next()?;
    let message = fields.next()?.to_owned();
    if !bounded_text(&message) {
        return None;
    }
    let mut severity = None;
    let mut rule = None;
    let mut suggestion = None;
    let mut metadata = Vec::new();
    for field in fields.take(MAX_QA_METADATA) {
        let (key, value) = field.split_once('=')?;
        match key {
            "severity" if bounded_text(value) => severity = Some(value.to_owned()),
            "rule" if bounded_text(value) => rule = Some(value.to_owned()),
            "suggestion" if bounded_text(value) => suggestion = Some(value.to_owned()),
            _ => {
                if let Ok(value) = QaMetadata::new(key.to_owned(), value.to_owned()) {
                    metadata.push(value);
                }
            }
        }
    }
    finding_from_text(
        file,
        finding_status(status_text),
        severity,
        message,
        rule,
        suggestion,
        metadata,
        line,
    )
}

fn parse_bitbake_line(file: &ExactFile, line: &str) -> Option<QaFinding> {
    let (status, severity, rest) = if let Some(rest) = line.strip_prefix("ERROR: QA Issue: ") {
        (QaFindingStatus::Failed, Some("error".into()), rest)
    } else {
        let rest = line.strip_prefix("WARNING: QA Issue: ")?;
        (QaFindingStatus::Warning, Some("warning".into()), rest)
    };
    let (message, rule) = rest
        .strip_suffix(']')
        .and_then(|value| value.rsplit_once(" ["))
        .map_or((rest.to_owned(), None), |(message, rule)| {
            (
                message.to_owned(),
                bounded_text(rule).then(|| rule.to_owned()),
            )
        });
    finding_from_text(
        file,
        status,
        severity,
        message,
        rule,
        None,
        Vec::new(),
        line,
    )
}

#[allow(clippy::too_many_arguments)]
fn finding_from_text(
    file: &ExactFile,
    status: QaFindingStatus,
    severity: Option<String>,
    message: String,
    rule: Option<String>,
    suggestion: Option<String>,
    metadata: Vec<QaMetadata>,
    raw: &str,
) -> Option<QaFinding> {
    if !bounded_text(&message) {
        return None;
    }
    let identity =
        QaFindingIdentity::new(file.producer.clone(), fingerprint(raw.as_bytes())).ok()?;
    Some(QaFinding {
        identity,
        status,
        severity,
        message,
        scope: file.scope.clone(),
        task: file.task.clone(),
        test_name: file.test_name.clone(),
        source: None,
        rule,
        suggestion,
        metadata,
    })
    .filter(QaFinding::is_valid)
}

fn parse_xml_findings(
    file: &ExactFile,
    bytes: &[u8],
    limitations: &mut Vec<String>,
) -> Result<Vec<QaFinding>, QaReportAdapterError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| QaReportAdapterError::MalformedReport(file.path.clone()))?;
    if !text.trim_start().starts_with("<qa-report") {
        push_limitation(
            limitations,
            "XML root was not the documented qa-report envelope".into(),
        );
        return Ok(Vec::new());
    }
    let mut findings = Vec::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("<finding ") {
        remaining = &remaining[start + "<finding ".len()..];
        let Some(end) = remaining.find("/>") else {
            push_limitation(limitations, "unterminated XML finding was ignored".into());
            break;
        };
        let attributes = &remaining[..end];
        remaining = &remaining[end + 2..];
        let values = parse_xml_attributes(attributes);
        let Some(status) = values.get("status") else {
            push_limitation(limitations, "XML finding without status was ignored".into());
            continue;
        };
        let Some(message) = values.get("message").filter(|value| bounded_text(value)) else {
            push_limitation(
                limitations,
                "XML finding without message was ignored".into(),
            );
            continue;
        };
        let raw = format!("{status}\0{message}\0{attributes}");
        if let Some(finding) = finding_from_text(
            file,
            finding_status(status),
            values.get("severity").cloned(),
            message.clone(),
            values.get("rule").cloned(),
            values.get("suggestion").cloned(),
            Vec::new(),
            &raw,
        ) {
            findings.push(finding);
        }
        if findings.len() >= MAX_QA_FINDINGS {
            push_limitation(
                limitations,
                format!("QA XML reached the {MAX_QA_FINDINGS}-record bound"),
            );
            break;
        }
    }
    if findings.is_empty() {
        push_limitation(
            limitations,
            "XML report contained no supported self-closing finding records".into(),
        );
    }
    Ok(findings)
}

fn parse_xml_attributes(value: &str) -> BTreeMap<String, String> {
    let mut output = BTreeMap::new();
    let mut remaining = value.trim();
    while let Some((key, tail)) = remaining.split_once('=') {
        let key = key.trim();
        let Some(tail) = tail.strip_prefix('"') else {
            break;
        };
        let Some(end) = tail.find('"') else {
            break;
        };
        let decoded = decode_xml(&tail[..end]);
        if bounded_token(key) && bounded_text(&decoded) {
            output.insert(key.to_owned(), decoded);
        }
        remaining = tail[end + 1..].trim_start();
    }
    output
}

fn decode_xml(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn bounded_json_string(
    object: &Map<String, Value>,
    key: &str,
    limitations: &mut Vec<String>,
) -> Option<String> {
    let value = match object.get(key)? {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        _ => return None,
    };
    if bounded_text(&value) {
        Some(value)
    } else {
        push_limitation(
            limitations,
            format!("invalid or oversized QA field was ignored: {key}"),
        );
        None
    }
}

fn scalar_metadata(object: &Map<String, Value>, limitations: &mut Vec<String>) -> Vec<QaMetadata> {
    let mut output = BTreeMap::new();
    for (key, value) in object.iter().take(MAX_QA_METADATA) {
        let value = match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            _ => None,
        };
        if let Some(value) = value {
            if let Ok(metadata) = QaMetadata::new(key.clone(), value) {
                output.insert(metadata.key.clone(), metadata);
            } else {
                push_limitation(limitations, "invalid QA metadata was ignored".into());
            }
        }
    }
    output.into_values().collect()
}

fn finding_status(value: &str) -> QaFindingStatus {
    match value.trim().to_ascii_lowercase().as_str() {
        "pass" | "passed" | "ok" => QaFindingStatus::Passed,
        "warn" | "warning" => QaFindingStatus::Warning,
        "fail" | "failed" | "error" => QaFindingStatus::Failed,
        "skip" | "skipped" => QaFindingStatus::Skipped,
        _ => QaFindingStatus::Unknown,
    }
}

fn bounded_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_QA_TEXT_BYTES && !value.chars().any(char::is_control)
}

fn bounded_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !matches!(value, "." | "..")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
        })
}

fn push_limitation(limitations: &mut Vec<String>, value: String) {
    if limitations.len() < MAX_QA_LIMITATIONS && bounded_text(&value) {
        limitations.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use yoctui_model::{QaLayerIdentity, QaScope, RecipeIdentity};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-qa-report-{}-{}",
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

    fn recipe_scope(root: &Path) -> QaFindingScope {
        let provider = root.join("busybox.bb");
        fs::write(&provider, "SUMMARY = \"BusyBox\"\n").unwrap();
        QaFindingScope::Recipe(
            QaScope::new(RecipeIdentity {
                name: "busybox".into(),
                file: provider,
            })
            .unwrap(),
        )
    }

    fn layer_scope(root: &Path) -> QaFindingScope {
        let layer = root.join("meta-demo");
        fs::create_dir_all(&layer).unwrap();
        QaFindingScope::Layer(QaLayerIdentity::new("meta-demo".into(), layer).unwrap())
    }

    fn candidate(
        path: PathBuf,
        format: Option<QaReportFormat>,
        producer: QaCheckId,
        scope: QaFindingScope,
    ) -> QaReportCandidate {
        let (task, test_name) = match scope {
            QaFindingScope::Recipe(_) => (Some("do_package_qa".into()), None),
            QaFindingScope::Layer(_) => (None, Some("bsp.test".into())),
        };
        QaReportCandidate {
            path,
            origin: QaReportOrigin::Managed,
            format,
            producer,
            scope,
            task,
            test_name,
        }
    }

    fn input(root: &Path, candidates: Vec<QaReportCandidate>) -> QaReportScanInput {
        let mut paths = candidates
            .iter()
            .map(|candidate| candidate.path.clone())
            .collect::<Vec<_>>();
        paths.sort();
        let mut checks = candidates
            .iter()
            .map(|candidate| candidate.producer.clone())
            .collect::<Vec<_>>();
        checks.sort();
        checks.dedup();
        let mut scopes = candidates
            .iter()
            .map(|candidate| candidate.scope.clone())
            .collect::<Vec<_>>();
        scopes.sort_by_key(|scope| format!("{scope:?}"));
        scopes.dedup();
        QaReportScanInput {
            build_directory: root.into(),
            request: QaReportRequest::new(1, paths).unwrap(),
            candidates,
            known_checks: checks,
            known_scopes: scopes,
        }
    }

    #[tokio::test]
    async fn qa_report_parses_exact_json_text_xml_and_bitbake_records() {
        let root = TestDirectory::new();
        let recipe = recipe_scope(&root.0);
        let layer = layer_scope(&root.0);
        let check = QaCheckId::new("recipe-package-busybox".into()).unwrap();
        let layer_check = QaCheckId::new("layer-meta-demo".into()).unwrap();
        let source = root.0.join("source.bbclass");
        fs::write(&source, "# source\n").unwrap();
        let json = root.0.join("report.json");
        fs::write(
            &json,
            format!(
                r#"{{"metadata":{{"tool":"oeqa"}},"findings":[{{"status":"failed","severity":"error","message":"license checksum mismatch","rule":"license-checksum","suggestion":"refresh LIC_FILES_CHKSUM","source":{{"path":"{}","line":7}},"metadata":{{"package":"busybox"}}}}]}}"#,
                source.display()
            ),
        )
        .unwrap();
        let text = root.0.join("report.qa");
        fs::write(
            &text,
            "warning\tpatch has fuzz\tseverity=warning\trule=patch-fuzz\n",
        )
        .unwrap();
        let xml = root.0.join("report.xml");
        fs::write(
            &xml,
            r#"<qa-report><finding status="passed" message="layer compatible" rule="compat"/></qa-report>"#,
        )
        .unwrap();
        let log = root.0.join("log.do_package_qa.log");
        fs::write(
            &log,
            "NOTE: ignored\nERROR: QA Issue: installed-vs-shipped mismatch [installed-vs-shipped]\n",
        )
        .unwrap();
        let response = QaReportAdapter::new()
            .scan(input(
                &root.0,
                vec![
                    candidate(
                        json,
                        Some(QaReportFormat::Json),
                        check.clone(),
                        recipe.clone(),
                    ),
                    candidate(
                        text,
                        Some(QaReportFormat::Text),
                        check.clone(),
                        recipe.clone(),
                    ),
                    candidate(xml, Some(QaReportFormat::Xml), layer_check, layer),
                    candidate(log, Some(QaReportFormat::BitBakeLog), check, recipe),
                ],
            ))
            .await
            .unwrap();
        assert_eq!(response.outcome.reports().len(), 4);
        assert_eq!(
            response
                .outcome
                .reports()
                .iter()
                .map(|report| report.findings.len())
                .sum::<usize>(),
            4
        );
        assert!(
            response
                .outcome
                .limitations()
                .iter()
                .any(|value| value.contains("record 1"))
        );
        assert_eq!(
            response.outcome.reports()[0].identity.producer,
            Some(QaCheckId("recipe-package-busybox".into()))
        );
    }

    #[tokio::test]
    async fn qa_report_directory_scan_is_bounded_exact_and_partial() {
        let root = TestDirectory::new();
        let reports = root.0.join("reports");
        fs::create_dir(&reports).unwrap();
        fs::write(
            reports.join("valid.json"),
            r#"[{"status":"passed","message":"URI is reachable"}]"#,
        )
        .unwrap();
        fs::write(reports.join("bad.json"), "{").unwrap();
        fs::write(reports.join("ignored.bin"), "not a report").unwrap();
        let scope = recipe_scope(&root.0);
        let check = QaCheckId::new("uri-fetch-busybox".into()).unwrap();
        let response = QaReportAdapter::new()
            .scan(input(&root.0, vec![candidate(reports, None, check, scope)]))
            .await
            .unwrap();
        assert_eq!(response.outcome.reports().len(), 1);
        assert!(matches!(
            response.outcome,
            QaReportScanOutcome::Partial { .. }
        ));
        assert!(
            response
                .outcome
                .limitations()
                .iter()
                .any(|value| value.contains("unsupported"))
        );
    }

    #[tokio::test]
    async fn qa_report_preserves_empty_and_malformed_as_distinct_outcomes() {
        let root = TestDirectory::new();
        let empty = root.0.join("empty");
        fs::create_dir(&empty).unwrap();
        let scope = recipe_scope(&root.0);
        let check = QaCheckId::new("patch-busybox".into()).unwrap();
        let response = QaReportAdapter::new()
            .scan(input(
                &root.0,
                vec![candidate(empty, None, check.clone(), scope.clone())],
            ))
            .await
            .unwrap();
        assert!(matches!(response.outcome, QaReportScanOutcome::Empty));

        let malformed = root.0.join("malformed.json");
        fs::write(&malformed, "{").unwrap();
        assert!(matches!(
            QaReportAdapter::new()
                .scan(input(
                    &root.0,
                    vec![candidate(
                        malformed.clone(),
                        Some(QaReportFormat::Json),
                        check,
                        scope
                    )],
                ))
                .await,
            Err(QaReportAdapterError::MalformedReport(path)) if path == malformed
        ));
    }

    #[tokio::test]
    async fn qa_report_rejects_missing_symlink_escape_duplicate_and_mismatch() {
        let root = TestDirectory::new();
        let scope = recipe_scope(&root.0);
        let check = QaCheckId::new("license-busybox".into()).unwrap();
        let missing = root.0.join("missing.json");
        assert!(matches!(
            QaReportAdapter::new()
                .scan(input(
                    &root.0,
                    vec![candidate(
                        missing.clone(),
                        Some(QaReportFormat::Json),
                        check.clone(),
                        scope.clone()
                    )],
                ))
                .await,
            Err(QaReportAdapterError::MissingPath(path)) if path == missing
        ));

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let real = root.0.join("real.json");
            let link = root.0.join("link.json");
            fs::write(&real, "[]").unwrap();
            symlink(&real, &link).unwrap();
            assert!(matches!(
                QaReportAdapter::new()
                    .scan(input(
                        &root.0,
                        vec![candidate(
                            link.clone(),
                            Some(QaReportFormat::Json),
                            check.clone(),
                            scope.clone()
                        )],
                    ))
                    .await,
                Err(QaReportAdapterError::SymlinkPath(path)) if path == link
            ));
        }

        let outside = TestDirectory::new();
        let escaped = outside.0.join("report.json");
        fs::write(&escaped, "[]").unwrap();
        assert!(matches!(
            QaReportAdapter::new()
                .scan(input(
                    &root.0,
                    vec![candidate(
                        escaped.clone(),
                        Some(QaReportFormat::Json),
                        check.clone(),
                        scope.clone()
                    )],
                ))
                .await,
            Err(QaReportAdapterError::EscapePath(path)) if path == escaped
        ));

        let valid = root.0.join("valid.json");
        fs::write(&valid, "[]").unwrap();
        let duplicate = candidate(valid.clone(), Some(QaReportFormat::Json), check, scope);
        let mut bad = input(&root.0, vec![duplicate.clone()]);
        bad.candidates.push(duplicate);
        assert!(matches!(
            QaReportAdapter::new().scan(bad).await,
            Err(QaReportAdapterError::InvalidRequest(_))
        ));
    }

    #[tokio::test]
    async fn qa_report_supports_imports_and_revalidates_stale_identity() {
        let build = TestDirectory::new();
        let imported = TestDirectory::new();
        let scope = recipe_scope(&build.0);
        let check = QaCheckId::new("recipe-package-busybox".into()).unwrap();
        let report = imported.0.join("import.json");
        fs::write(
            &report,
            r#"[{"status":"passed","message":"package QA passed"}]"#,
        )
        .unwrap();
        let mut exact = candidate(report.clone(), Some(QaReportFormat::Json), check, scope);
        exact.origin = QaReportOrigin::Import;
        let response = QaReportAdapter::new()
            .scan(input(&build.0, vec![exact]))
            .await
            .unwrap();
        let identity = response.outcome.reports()[0].identity.clone();
        QaReportAdapter::new().revalidate(&identity).unwrap();
        fs::write(&report, r#"[{"status":"failed","message":"changed"}]"#).unwrap();
        assert!(matches!(
            QaReportAdapter::new().revalidate(&identity),
            Err(QaReportAdapterError::StaleReport(path)) if path == report
        ));
    }

    #[tokio::test]
    async fn qa_report_preserves_cancellation_timeout_loss_and_oversize() {
        let root = TestDirectory::new();
        let scope = recipe_scope(&root.0);
        let check = QaCheckId::new("recipe-package-busybox".into()).unwrap();
        let report = root.0.join("report.json");
        fs::write(&report, "[]").unwrap();
        let scan = input(
            &root.0,
            vec![candidate(
                report.clone(),
                Some(QaReportFormat::Json),
                check.clone(),
                scope.clone(),
            )],
        );
        let cancellation = QaReportCancellation::default();
        cancellation.cancel();
        assert!(matches!(
            QaReportAdapter::new()
                .scan_with_cancellation(scan.clone(), cancellation)
                .await,
            Err(QaReportAdapterError::Cancelled)
        ));
        assert!(matches!(
            QaReportAdapter::new()
                .with_timeout(Duration::ZERO)
                .scan(scan.clone())
                .await,
            Err(QaReportAdapterError::Timeout(0))
        ));
        assert!(matches!(
            QaReportAdapter::new().with_worker_panic().scan(scan).await,
            Err(QaReportAdapterError::WorkerLost(_))
        ));

        let oversized = root.0.join("large.json");
        fs::File::create(&oversized)
            .unwrap()
            .set_len(MAX_QA_FILE_BYTES + 1)
            .unwrap();
        assert!(matches!(
            QaReportAdapter::new()
                .scan(input(
                    &root.0,
                    vec![candidate(
                        oversized.clone(),
                        Some(QaReportFormat::Json),
                        check,
                        scope
                    )],
                ))
                .await,
            Err(QaReportAdapterError::OversizedReport(path)) if path == oversized
        ));
    }
}
