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
    CveFinding, CveFindingIdentity, CveReport, CveStatus, MAX_SECURITY_COMPONENTS,
    MAX_SECURITY_FINDINGS, MAX_SECURITY_LIMITATIONS, MAX_SECURITY_METADATA, MAX_SECURITY_REPORTS,
    MAX_SECURITY_TEXT_BYTES, SecurityMetadata, SecurityReport, SecurityReportIdentity,
    SecurityReportRequest, SpdxArtifactKind, SpdxComponent, SpdxDocument,
    normalize_security_reports,
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

fn validate_request(request: &SecurityReportRequest) -> Result<(), SecurityReportAdapterError> {
    let normalized = SecurityReportRequest::new(request.generation, request.paths.clone())
        .map_err(|message| SecurityReportAdapterError::InvalidRequest(message.into()))?;
    if &normalized != request {
        return Err(SecurityReportAdapterError::InvalidRequest(
            "paths must be sorted and unique".into(),
        ));
    }
    Ok(())
}

fn scan_reports(
    request: SecurityReportRequest,
    cancellation: SecurityReportCancellation,
    deadline: Instant,
) -> Result<SecurityReportResponse, SecurityReportAdapterError> {
    let roots = request
        .paths
        .iter()
        .map(|path| validate_explicit_path(path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut files = BTreeSet::new();
    let mut limitations = Vec::new();
    let mut failures = ScanFailures::default();
    let mut directory_count = 0_usize;

    for root in roots {
        check_control(&cancellation, deadline)?;
        let metadata = fs::symlink_metadata(&root).map_err(|error| path_error(&root, error))?;
        if metadata.is_file() {
            files.insert(root);
        } else {
            collect_directory_files(
                &root,
                &root,
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
    for path in files {
        check_control(&cancellation, deadline)?;
        if reports.len() >= MAX_SECURITY_REPORTS {
            push_limitation(
                &mut limitations,
                format!(
                    "Security report {} was omitted at the {MAX_SECURITY_REPORTS}-report bound",
                    path.display()
                ),
            );
            continue;
        }
        let before = fs::symlink_metadata(&path).map_err(|error| path_error(&path, error))?;
        if before.file_type().is_symlink() || !before.is_file() {
            push_limitation(
                &mut limitations,
                format!(
                    "Security report became unsafe before parsing: {}",
                    path.display()
                ),
            );
            continue;
        }
        let size = before.len();
        if size == 0 {
            failures.malformed.get_or_insert_with(|| path.clone());
            push_limitation(
                &mut limitations,
                format!("empty Security report was ignored: {}", path.display()),
            );
            continue;
        }
        if size > MAX_SECURITY_FILE_BYTES {
            failures.oversized.get_or_insert_with(|| path.clone());
            push_limitation(
                &mut limitations,
                format!(
                    "Security report exceeded the {MAX_SECURITY_FILE_BYTES}-byte per-file bound: {}",
                    path.display()
                ),
            );
            continue;
        }
        if total_bytes.saturating_add(size) > MAX_SECURITY_TOTAL_BYTES {
            failures.oversized.get_or_insert_with(|| path.clone());
            push_limitation(
                &mut limitations,
                format!(
                    "Security report was omitted at the {MAX_SECURITY_TOTAL_BYTES}-byte total bound: {}",
                    path.display()
                ),
            );
            continue;
        }
        let Some(modified_at) = before.modified().ok() else {
            push_limitation(
                &mut limitations,
                format!(
                    "Security report modification time was unavailable: {}",
                    path.display()
                ),
            );
            continue;
        };
        let bytes = read_bounded_file(&path, size, &cancellation, deadline)?;
        total_bytes = total_bytes.saturating_add(bytes.len() as u64);
        let after = fs::symlink_metadata(&path).map_err(|error| path_error(&path, error))?;
        if after.file_type().is_symlink()
            || !after.is_file()
            || after.len() != size
            || after.modified().ok() != Some(modified_at)
            || fs::canonicalize(&path).ok().as_ref() != Some(&path)
        {
            failures.stale.get_or_insert_with(|| path.clone());
            push_limitation(
                &mut limitations,
                format!(
                    "Security report changed while it was acquired and was ignored: {}",
                    path.display()
                ),
            );
            continue;
        }
        let fingerprint = format!("{:x}", Sha256::digest(&bytes));
        let identity = SecurityReportIdentity::new(path.clone(), size, modified_at, fingerprint)
            .map_err(|message| SecurityReportAdapterError::Io(message.into()))?;
        match parse_report(identity, &bytes, &mut limitations, &cancellation, deadline)? {
            ParseReportOutcome::Report(report) => reports.push(*report),
            ParseReportOutcome::Malformed => {
                failures.malformed.get_or_insert_with(|| path.clone());
                push_limitation(
                    &mut limitations,
                    format!("malformed Security report was ignored: {}", path.display()),
                );
            }
            ParseReportOutcome::Unsupported => {
                failures.unsupported.get_or_insert_with(|| path.clone());
                push_limitation(
                    &mut limitations,
                    format!(
                        "unsupported Security report was ignored: {}",
                        path.display()
                    ),
                );
            }
        }
    }

    let (reports, model_limitations) = normalize_security_reports(reports);
    for limitation in model_limitations {
        push_limitation(&mut limitations, limitation);
    }
    for report in &reports {
        let (path, report_limitations) = match report {
            SecurityReport::Cve(report) => (&report.identity.path, &report.limitations),
            SecurityReport::Spdx(report) => (&report.identity.path, &report.limitations),
        };
        for limitation in report_limitations {
            push_limitation(
                &mut limitations,
                format!("{}: {limitation}", path.display()),
            );
        }
    }
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_SECURITY_LIMITATIONS);
    if reports.is_empty() && !limitations.is_empty() {
        return Err(failures.terminal_error(&limitations));
    }
    let outcome = if reports.is_empty() {
        SecurityReportScanOutcome::Empty
    } else if limitations.is_empty() {
        SecurityReportScanOutcome::Complete(reports)
    } else {
        SecurityReportScanOutcome::Partial {
            reports,
            limitations,
        }
    };
    Ok(SecurityReportResponse { request, outcome })
}

#[allow(clippy::too_many_arguments)]
fn collect_directory_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeSet<PathBuf>,
    directory_count: &mut usize,
    limitations: &mut Vec<String>,
    failures: &mut ScanFailures,
    cancellation: &SecurityReportCancellation,
    deadline: Instant,
) -> Result<(), SecurityReportAdapterError> {
    if *directory_count >= MAX_SECURITY_DIRECTORIES {
        push_limitation(
            limitations,
            format!(
                "Security directory was omitted at the {MAX_SECURITY_DIRECTORIES}-directory bound: {}",
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
        let entry = match entry {
            Ok(entry) => entry.path(),
            Err(error) => {
                push_limitation(
                    limitations,
                    format!("a Security directory entry was unreadable: {error}"),
                );
                continue;
            }
        };
        entries.insert(entry);
        if entries.len() > MAX_SECURITY_DIRECTORY_ENTRIES {
            entries.pop_last();
            omitted += 1;
        }
    }
    if omitted > 0 {
        push_limitation(
            limitations,
            format!(
                "{omitted} Security entries were omitted from {} at the {MAX_SECURITY_DIRECTORY_ENTRIES}-entry bound",
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
                        "Security entry metadata was unavailable for {}: {error}",
                        path.display()
                    ),
                );
                continue;
            }
        };
        if metadata.file_type().is_symlink() {
            failures.symlink.get_or_insert_with(|| path.clone());
            push_limitation(
                limitations,
                format!("Security symlink was not followed: {}", path.display()),
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
                failures.escape.get_or_insert_with(|| path.clone());
                push_limitation(
                    limitations,
                    format!(
                        "Security entry escaped its explicit root or was not canonical: {}",
                        path.display()
                    ),
                );
                continue;
            }
        };
        if metadata.is_dir() {
            collect_directory_files(
                root,
                &canonical,
                files,
                directory_count,
                limitations,
                failures,
                cancellation,
                deadline,
            )?;
        } else if metadata.is_file() {
            if is_candidate(&canonical) {
                files.insert(canonical);
            } else {
                failures.unsupported.get_or_insert_with(|| path.clone());
                push_limitation(
                    limitations,
                    format!("unsupported Security entry was ignored: {}", path.display()),
                );
            }
        } else {
            push_limitation(
                limitations,
                format!("non-regular Security entry was ignored: {}", path.display()),
            );
        }
    }
    Ok(())
}

fn validate_explicit_path(path: &Path) -> Result<PathBuf, SecurityReportAdapterError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| path_error(path, error))?;
    if metadata.file_type().is_symlink() {
        return Err(SecurityReportAdapterError::SymlinkPath(path.into()));
    }
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(SecurityReportAdapterError::UnsafePath(path.into()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| SecurityReportAdapterError::Io(error.to_string()))?;
    if canonical != path || canonical == Path::new("/") {
        return Err(SecurityReportAdapterError::EscapePath(path.into()));
    }
    if metadata.is_file() && !is_candidate(&canonical) {
        return Err(SecurityReportAdapterError::UnsupportedPath(path.into()));
    }
    Ok(canonical)
}

fn path_error(path: &Path, error: io::Error) -> SecurityReportAdapterError {
    match error.kind() {
        io::ErrorKind::NotFound => SecurityReportAdapterError::MissingPath(path.into()),
        io::ErrorKind::PermissionDenied => {
            SecurityReportAdapterError::PermissionDenied(path.into())
        }
        _ => SecurityReportAdapterError::Io(format!("{}: {error}", path.display())),
    }
}

fn read_bounded_file(
    path: &Path,
    expected_size: u64,
    cancellation: &SecurityReportCancellation,
    deadline: Instant,
) -> Result<Vec<u8>, SecurityReportAdapterError> {
    let mut file = fs::File::open(path).map_err(|error| path_error(path, error))?;
    let mut bytes = Vec::with_capacity(expected_size as usize);
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_control(cancellation, deadline)?;
        let read = file
            .read(&mut buffer)
            .map_err(|error| SecurityReportAdapterError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read) > MAX_SECURITY_FILE_BYTES as usize {
            return Err(SecurityReportAdapterError::Io(format!(
                "Security report grew beyond its bound: {}",
                path.display()
            )));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn is_candidate(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    name.ends_with(".json")
        || name.ends_with(".jsonl")
        || name.ends_with(".cve")
        || name.ends_with(".cve.txt")
        || name.ends_with(".cve.log")
        || name.ends_with(".spdx.tar.zst")
        || name.ends_with(".spdx.tar.gz")
        || name.ends_with(".spdx.zip")
}

enum ParseReportOutcome {
    Report(Box<SecurityReport>),
    Malformed,
    Unsupported,
}

fn parse_report(
    identity: SecurityReportIdentity,
    bytes: &[u8],
    limitations: &mut Vec<String>,
    cancellation: &SecurityReportCancellation,
    deadline: Instant,
) -> Result<ParseReportOutcome, SecurityReportAdapterError> {
    check_control(cancellation, deadline)?;
    let name = identity
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name.ends_with(".spdx.tar.zst")
        || name.ends_with(".spdx.tar.gz")
        || name.ends_with(".spdx.zip")
    {
        return Ok(ParseReportOutcome::Report(Box::new(SecurityReport::Spdx(
            SpdxDocument {
                identity,
                scope: None,
                kind: SpdxArtifactKind::Archive,
                spdx_version: None,
                name: None,
                namespace: None,
                data_license: None,
                creators: Vec::new(),
                components: Vec::new(),
                file_count: None,
                relationship_count: None,
                checksums: Vec::new(),
                limitations: vec![
                    "SPDX archive is retained as an exact artifact; archive contents are not parsed"
                        .into(),
                ],
            },
        ))));
    }
    if name.ends_with(".cve") || name.ends_with(".cve.txt") || name.ends_with(".cve.log") {
        return parse_cve_text(identity, bytes, limitations);
    }
    let value: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(error) => {
            push_limitation(
                limitations,
                format!("JSON report could not be parsed: {error}"),
            );
            return Ok(ParseReportOutcome::Malformed);
        }
    };
    if looks_like_spdx(&value) || name.contains("spdx") {
        return Ok(ParseReportOutcome::Report(Box::new(SecurityReport::Spdx(
            parse_spdx(identity, &value),
        ))));
    }
    if looks_like_cve(&value) || name.contains("cve") {
        return Ok(ParseReportOutcome::Report(Box::new(SecurityReport::Cve(
            parse_cve_json(identity, &value),
        ))));
    }
    Ok(ParseReportOutcome::Unsupported)
}

fn looks_like_spdx(value: &Value) -> bool {
    value.get("spdxVersion").is_some()
        || value.get("SPDXID").is_some()
        || value.get("documentNamespace").is_some()
        || value.get("specVersion").is_some()
        || value.get("@context").is_some()
}

fn looks_like_cve(value: &Value) -> bool {
    match value {
        Value::Array(values) => values.iter().any(looks_like_cve),
        Value::Object(object) => {
            ["cves", "CVE", "cve", "issues", "issue"]
                .iter()
                .any(|key| object.contains_key(*key))
                || cve_id(object).is_some()
                || object.get("package").is_some_and(|value| value.is_array())
        }
        _ => false,
    }
}

#[derive(Debug, Clone, Default)]
struct CveContext {
    recipe: Option<String>,
    package: Option<String>,
    product: Option<String>,
    version: Option<String>,
}

fn parse_cve_json(identity: SecurityReportIdentity, value: &Value) -> CveReport {
    let mut findings = Vec::new();
    let mut limitations = Vec::new();
    let mut metadata = Vec::new();
    if let Value::Object(object) = value {
        metadata = scalar_metadata(
            object,
            &["package", "packages", "products", "cves", "issues"],
        );
    }
    collect_cve_findings(
        value,
        &CveContext::default(),
        0,
        &mut findings,
        &mut limitations,
    );
    if findings.is_empty() {
        push_limitation(
            &mut limitations,
            "CVE JSON contained no supported findings".into(),
        );
    }
    CveReport {
        identity,
        scope: None,
        findings,
        metadata,
        limitations,
    }
}

fn collect_cve_findings(
    value: &Value,
    inherited: &CveContext,
    depth: usize,
    findings: &mut Vec<CveFinding>,
    limitations: &mut Vec<String>,
) {
    if depth > MAX_PARSE_DEPTH {
        push_limitation(
            limitations,
            format!("CVE JSON exceeded the {MAX_PARSE_DEPTH}-level nesting bound"),
        );
        return;
    }
    match value {
        Value::Array(values) => {
            for value in values.iter().take(MAX_SECURITY_FINDINGS) {
                collect_cve_findings(value, inherited, depth + 1, findings, limitations);
            }
            if values.len() > MAX_SECURITY_FINDINGS {
                push_limitation(
                    limitations,
                    format!(
                        "{} CVE values were omitted at the {MAX_SECURITY_FINDINGS}-record bound",
                        values.len() - MAX_SECURITY_FINDINGS
                    ),
                );
            }
        }
        Value::Object(object) => {
            let context = cve_context(object, inherited, limitations);
            if cve_id(object).is_some() {
                if findings.len() >= MAX_SECURITY_FINDINGS {
                    push_limitation(
                        limitations,
                        format!("CVE findings reached the {MAX_SECURITY_FINDINGS}-record bound"),
                    );
                    return;
                }
                match cve_finding(object, &context, limitations) {
                    Some(finding) => findings.push(finding),
                    None => {
                        push_limitation(limitations, "a malformed CVE finding was ignored".into())
                    }
                }
            }
            for key in [
                "package", "packages", "products", "cves", "CVEs", "issues", "issue",
            ] {
                if let Some(child) = object.get(key) {
                    collect_cve_findings(child, &context, depth + 1, findings, limitations);
                }
            }
        }
        _ => {}
    }
}

fn cve_context(
    object: &Map<String, Value>,
    inherited: &CveContext,
    limitations: &mut Vec<String>,
) -> CveContext {
    let name = bounded_json_string(object, &["name"], limitations);
    CveContext {
        recipe: bounded_json_string(object, &["recipe", "pn"], limitations)
            .or_else(|| inherited.recipe.clone())
            .or_else(|| name.clone()),
        package: bounded_json_string(object, &["package", "package_name"], limitations)
            .or_else(|| inherited.package.clone())
            .or(name),
        product: bounded_json_string(object, &["product"], limitations)
            .or_else(|| inherited.product.clone()),
        version: bounded_json_string(object, &["version", "package_version"], limitations)
            .or_else(|| inherited.version.clone()),
    }
}

fn cve_id(object: &Map<String, Value>) -> Option<String> {
    ["id", "cve", "CVE"]
        .iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::to_owned)
        .filter(|value| value.starts_with("CVE-"))
}

fn cve_finding(
    object: &Map<String, Value>,
    context: &CveContext,
    limitations: &mut Vec<String>,
) -> Option<CveFinding> {
    let cve = cve_id(object)?;
    let recipe = bounded_json_string(object, &["recipe", "pn"], limitations)
        .or_else(|| context.recipe.clone())?;
    let package = bounded_json_string(object, &["package", "package_name"], limitations)
        .or_else(|| context.package.clone());
    let identity = CveFindingIdentity::new(cve, recipe, package).ok()?;
    let raw_status = bounded_json_string(object, &["status", "state"], limitations);
    let status = raw_status
        .as_deref()
        .map(cve_status)
        .unwrap_or(CveStatus::Unknown);
    if status == CveStatus::Unknown && raw_status.is_some() {
        push_limitation(
            limitations,
            format!(
                "unrecognized CVE status was preserved as Unknown: {}",
                raw_status.unwrap_or_default()
            ),
        );
    }
    let mapping = object
        .get("mapping")
        .and_then(Value::as_object)
        .map(|mapping| scalar_metadata(mapping, &[]))
        .unwrap_or_default();
    Some(CveFinding {
        identity,
        status,
        product: bounded_json_string(object, &["product"], limitations)
            .or_else(|| context.product.clone()),
        version: bounded_json_string(object, &["version", "package_version"], limitations)
            .or_else(|| context.version.clone()),
        severity: bounded_json_string(object, &["severity"], limitations),
        score: bounded_json_string(object, &["score", "cvss_score"], limitations),
        vector: bounded_json_string(object, &["vector", "cvss_vector"], limitations),
        advisory_url: bounded_json_string(object, &["link", "url", "advisory"], limitations)
            .filter(|value| value.starts_with("https://")),
        summary: bounded_json_string(object, &["summary", "description"], limitations),
        mapping,
    })
}

fn parse_cve_text(
    identity: SecurityReportIdentity,
    bytes: &[u8],
    outer_limitations: &mut Vec<String>,
) -> Result<ParseReportOutcome, SecurityReportAdapterError> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            push_limitation(
                outer_limitations,
                format!("CVE text was not UTF-8: {error}"),
            );
            return Ok(ParseReportOutcome::Malformed);
        }
    };
    let mut findings = Vec::new();
    let mut limitations = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = if line.contains('\t') {
            line.split('\t').collect::<Vec<_>>()
        } else {
            line.split_whitespace().collect::<Vec<_>>()
        };
        if fields.first().is_some_and(|field| {
            field.eq_ignore_ascii_case("recipe") || field.eq_ignore_ascii_case("pn")
        }) {
            continue;
        }
        let Some(cve_index) = fields.iter().position(|field| field.starts_with("CVE-")) else {
            push_limitation(
                &mut limitations,
                format!("CVE text line {} was ignored", index + 1),
            );
            continue;
        };
        if cve_index == 0 || fields.len() <= cve_index + 1 {
            push_limitation(
                &mut limitations,
                format!("malformed CVE text line {} was ignored", index + 1),
            );
            continue;
        }
        let recipe = fields[0].to_owned();
        let package = (cve_index > 2).then(|| fields[1].to_owned());
        let version = (cve_index > 1).then(|| fields[cve_index - 1].to_owned());
        let Ok(finding_identity) =
            CveFindingIdentity::new(fields[cve_index].to_owned(), recipe, package)
        else {
            push_limitation(
                &mut limitations,
                format!("invalid CVE identity on text line {}", index + 1),
            );
            continue;
        };
        findings.push(CveFinding {
            identity: finding_identity,
            status: cve_status(fields[cve_index + 1]),
            product: None,
            version,
            severity: fields.get(cve_index + 2).map(|value| (*value).to_owned()),
            score: fields.get(cve_index + 3).map(|value| (*value).to_owned()),
            vector: fields.get(cve_index + 4).map(|value| (*value).to_owned()),
            advisory_url: fields
                .get(cve_index + 5)
                .filter(|value| value.starts_with("https://"))
                .map(|value| (*value).to_owned()),
            summary: None,
            mapping: Vec::new(),
        });
        if findings.len() >= MAX_SECURITY_FINDINGS {
            push_limitation(
                &mut limitations,
                format!("CVE text reached the {MAX_SECURITY_FINDINGS}-record bound"),
            );
            break;
        }
    }
    Ok(ParseReportOutcome::Report(Box::new(SecurityReport::Cve(
        CveReport {
            identity,
            scope: None,
            findings,
            metadata: Vec::new(),
            limitations,
        },
    ))))
}

fn cve_status(value: &str) -> CveStatus {
    let normalized = value.trim().to_ascii_lowercase().replace([' ', '_'], "-");
    match normalized.as_str() {
        "vulnerable" | "unpatched" | "version-in-range" => CveStatus::Vulnerable,
        "patched" | "fixed" | "fix-file-included" => CveStatus::Patched,
        "ignored" => CveStatus::Ignored,
        "not-affected" | "not-applicable" | "version-not-in-range" => CveStatus::NotAffected,
        _ => CveStatus::Unknown,
    }
}

fn parse_spdx(identity: SecurityReportIdentity, value: &Value) -> SpdxDocument {
    let mut limitations = Vec::new();
    let Some(object) = value.as_object() else {
        return SpdxDocument {
            identity,
            scope: None,
            kind: SpdxArtifactKind::Json,
            spdx_version: None,
            name: None,
            namespace: None,
            data_license: None,
            creators: Vec::new(),
            components: Vec::new(),
            file_count: None,
            relationship_count: None,
            checksums: Vec::new(),
            limitations: vec![
                "unsupported SPDX JSON root was retained as an exact artifact".into(),
            ],
        };
    };
    let spdx_version =
        bounded_json_string(object, &["spdxVersion", "specVersion"], &mut limitations);
    if spdx_version.is_none() {
        push_limitation(
            &mut limitations,
            "unsupported SPDX schema was retained as an exact artifact".into(),
        );
    }
    let creators = object
        .get("creationInfo")
        .and_then(Value::as_object)
        .and_then(|creation| creation.get("creators"))
        .and_then(Value::as_array)
        .map(|values| bounded_string_array(values, &mut limitations))
        .unwrap_or_default();
    let components = object
        .get("packages")
        .or_else(|| object.get("components"))
        .and_then(Value::as_array)
        .map(|values| spdx_components(values, &mut limitations))
        .unwrap_or_default();
    let file_count = object
        .get("files")
        .and_then(Value::as_array)
        .map(|values| values.len() as u64);
    let relationship_count = object
        .get("relationships")
        .and_then(Value::as_array)
        .map(|values| values.len() as u64);
    let checksums = object
        .get("checksums")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| {
                    let object = value.as_object()?;
                    let algorithm = object.get("algorithm")?.as_str()?.to_owned();
                    let checksum = object.get("checksumValue")?.as_str()?.to_owned();
                    SecurityMetadata::new(algorithm, checksum).ok()
                })
                .take(MAX_SECURITY_METADATA)
                .collect()
        })
        .unwrap_or_default();
    SpdxDocument {
        identity,
        scope: None,
        kind: SpdxArtifactKind::Json,
        spdx_version,
        name: bounded_json_string(object, &["name"], &mut limitations),
        namespace: bounded_json_string(
            object,
            &["documentNamespace", "namespace"],
            &mut limitations,
        ),
        data_license: bounded_json_string(object, &["dataLicense"], &mut limitations),
        creators,
        components,
        file_count,
        relationship_count,
        checksums,
        limitations,
    }
}

fn spdx_components(values: &[Value], limitations: &mut Vec<String>) -> Vec<SpdxComponent> {
    let mut components = Vec::new();
    for value in values.iter().take(MAX_SECURITY_COMPONENTS) {
        let Some(object) = value.as_object() else {
            push_limitation(limitations, "a malformed SPDX component was ignored".into());
            continue;
        };
        let identity = bounded_json_string(object, &["SPDXID", "spdxId", "id"], limitations);
        let name = bounded_json_string(object, &["name"], limitations);
        let Some((identity, name)) = identity.zip(name) else {
            push_limitation(limitations, "a malformed SPDX component was ignored".into());
            continue;
        };
        let component = SpdxComponent {
            identity,
            name,
            version: bounded_json_string(object, &["versionInfo", "version"], limitations),
            supplier: bounded_json_string(object, &["supplier"], limitations),
            license: bounded_json_string(
                object,
                &["licenseConcluded", "licenseDeclared", "license"],
                limitations,
            ),
        };
        if component.is_valid() {
            components.push(component);
        } else {
            push_limitation(limitations, "an invalid SPDX component was ignored".into());
        }
    }
    if values.len() > MAX_SECURITY_COMPONENTS {
        push_limitation(
            limitations,
            format!(
                "{} SPDX components were omitted at the {MAX_SECURITY_COMPONENTS}-record bound",
                values.len() - MAX_SECURITY_COMPONENTS
            ),
        );
    }
    components
}

fn bounded_string_array(values: &[Value], limitations: &mut Vec<String>) -> Vec<String> {
    let mut output = Vec::new();
    for value in values.iter().take(MAX_SECURITY_METADATA) {
        if let Some(value) = value.as_str().filter(|value| valid_text(value)) {
            output.push(value.to_owned());
        } else {
            push_limitation(limitations, "an invalid SPDX text value was ignored".into());
        }
    }
    output
}

fn bounded_json_string(
    object: &Map<String, Value>,
    keys: &[&str],
    limitations: &mut Vec<String>,
) -> Option<String> {
    let value = keys
        .iter()
        .find_map(|key| object.get(*key))
        .and_then(|value| match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })?;
    if valid_text(&value) {
        Some(value)
    } else {
        push_limitation(
            limitations,
            format!(
                "an oversized or invalid Security field was ignored (maximum {MAX_SECURITY_TEXT_BYTES} bytes)"
            ),
        );
        None
    }
}

fn scalar_metadata(object: &Map<String, Value>, excluded: &[&str]) -> Vec<SecurityMetadata> {
    let mut metadata = BTreeMap::new();
    for (key, value) in object {
        if excluded.contains(&key.as_str()) {
            continue;
        }
        let value = match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            _ => None,
        };
        if let Some(value) = value
            && let Ok(value) = SecurityMetadata::new(key.clone(), value)
        {
            metadata.insert(value.key.clone(), value);
        }
        if metadata.len() >= MAX_SECURITY_METADATA {
            break;
        }
    }
    metadata.into_values().collect()
}

fn valid_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SECURITY_TEXT_BYTES
        && !value.chars().any(char::is_control)
}

fn push_limitation(limitations: &mut Vec<String>, value: String) {
    if limitations.len() < MAX_SECURITY_LIMITATIONS && valid_text(&value) {
        limitations.push(value);
    }
}

fn check_control(
    cancellation: &SecurityReportCancellation,
    deadline: Instant,
) -> Result<(), SecurityReportAdapterError> {
    if cancellation.is_cancelled() {
        Err(SecurityReportAdapterError::Cancelled)
    } else if Instant::now() >= deadline {
        Err(SecurityReportAdapterError::Timeout(0))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-security-report-{}-{}",
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

    fn request(path: &Path) -> SecurityReportRequest {
        SecurityReportRequest::new(1, vec![path.to_path_buf()]).unwrap()
    }

    fn write_cve(path: &Path) {
        fs::write(
            path,
            br#"{
              "version": "1",
              "packages": [{
                "name": "busybox",
                "version": "1.36",
                "products": [{
                  "product": "busybox",
                  "cves": [{
                    "id": "CVE-2024-1234",
                    "status": "Unpatched",
                    "severity": "HIGH",
                    "score": "8.1",
                    "vector": "CVSS:3.1/AV:N",
                    "link": "https://example.invalid/CVE-2024-1234",
                    "summary": "bounded finding",
                    "mapping": {"source": "cve-check"}
                  }]
                }]
              }]
            }"#,
        )
        .unwrap();
    }

    fn write_spdx(path: &Path) {
        fs::write(
            path,
            br#"{
              "spdxVersion": "SPDX-2.3",
              "SPDXID": "SPDXRef-DOCUMENT",
              "name": "core-image-minimal",
              "documentNamespace": "https://example.invalid/spdx/image",
              "dataLicense": "CC0-1.0",
              "creationInfo": {"creators": ["Tool: bitbake"]},
              "packages": [{
                "SPDXID": "SPDXRef-Package-busybox",
                "name": "busybox",
                "versionInfo": "1.36",
                "supplier": "Organization: Yocto",
                "licenseConcluded": "GPL-2.0-only"
              }],
              "files": [{}, {}],
              "relationships": [{}]
            }"#,
        )
        .unwrap();
    }

    #[tokio::test]
    async fn security_report_parses_cve_and_spdx_with_exact_identities() {
        let directory = TestDirectory::new();
        write_cve(&directory.path().join("busybox.cve.json"));
        write_spdx(&directory.path().join("image.spdx.json"));

        let response = SecurityReportAdapter::new()
            .scan(request(directory.path()))
            .await
            .unwrap();
        assert!(matches!(
            response.outcome,
            SecurityReportScanOutcome::Complete(_)
        ));
        let reports = response.outcome.reports();
        assert_eq!(reports.len(), 2);
        assert!(reports.iter().all(|report| {
            report.identity().path.starts_with(directory.path())
                && report.identity().fingerprint.len() == 64
        }));
        let cve = reports
            .iter()
            .find_map(|report| match report {
                SecurityReport::Cve(report) => Some(report),
                _ => None,
            })
            .unwrap();
        assert_eq!(cve.findings.len(), 1);
        assert_eq!(cve.findings[0].status, CveStatus::Vulnerable);
        assert_eq!(cve.findings[0].identity.recipe, "busybox");
        let spdx = reports
            .iter()
            .find_map(|report| match report {
                SecurityReport::Spdx(report) => Some(report),
                _ => None,
            })
            .unwrap();
        assert_eq!(spdx.components.len(), 1);
        assert_eq!(spdx.file_count, Some(2));
        assert_eq!(spdx.relationship_count, Some(1));
    }

    #[tokio::test]
    async fn security_report_distinguishes_empty_and_mixed_partial_scans() {
        let empty = TestDirectory::new();
        let response = SecurityReportAdapter::new()
            .scan(request(empty.path()))
            .await
            .unwrap();
        assert_eq!(response.outcome, SecurityReportScanOutcome::Empty);

        let mixed = TestDirectory::new();
        write_cve(&mixed.path().join("valid.cve.json"));
        fs::write(mixed.path().join("broken.cve.json"), b"{").unwrap();
        let response = SecurityReportAdapter::new()
            .scan(request(mixed.path()))
            .await
            .unwrap();
        assert!(matches!(
            response.outcome,
            SecurityReportScanOutcome::Partial {
                ref reports,
                ref limitations
            } if reports.len() == 1 && !limitations.is_empty()
        ));
    }

    #[tokio::test]
    async fn security_report_parses_text_and_retains_unsupported_spdx_artifacts() {
        let directory = TestDirectory::new();
        fs::write(
            directory.path().join("findings.cve.txt"),
            b"recipe package version cve status severity\nbusybox busybox 1.36 CVE-2024-1234 Patched HIGH\nbad row\n",
        )
        .unwrap();
        fs::write(
            directory.path().join("future.spdx.json"),
            br#"{"name":"future","components":[]}"#,
        )
        .unwrap();
        fs::write(directory.path().join("image.spdx.tar.zst"), b"archive").unwrap();

        let response = SecurityReportAdapter::new()
            .scan(request(directory.path()))
            .await
            .unwrap();
        assert_eq!(response.outcome.reports().len(), 3);
        assert!(!response.outcome.limitations().is_empty());
        assert!(response.outcome.reports().iter().any(|report| matches!(
            report,
            SecurityReport::Spdx(SpdxDocument {
                kind: SpdxArtifactKind::Archive,
                ..
            })
        )));
    }

    #[tokio::test]
    async fn security_report_rejects_wholly_malformed_and_oversized_inputs() {
        let directory = TestDirectory::new();
        let malformed = directory.path().join("bad.cve.json");
        fs::write(&malformed, b"{").unwrap();
        assert!(matches!(
            SecurityReportAdapter::new().scan(request(&malformed)).await,
            Err(SecurityReportAdapterError::MalformedReport(path)) if path == malformed
        ));

        let oversized = directory.path().join("huge.cve.json");
        let file = fs::File::create(&oversized).unwrap();
        file.set_len(MAX_SECURITY_FILE_BYTES + 1).unwrap();
        assert!(matches!(
            SecurityReportAdapter::new().scan(request(&oversized)).await,
            Err(SecurityReportAdapterError::OversizedReport(path)) if path == oversized
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn security_report_rejects_explicit_symlinks_and_does_not_follow_nested_ones() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new();
        let outside = TestDirectory::new();
        let report = outside.path().join("outside.cve.json");
        write_cve(&report);
        let link = directory.path().join("linked.cve.json");
        symlink(&report, &link).unwrap();
        assert!(matches!(
            SecurityReportAdapter::new().scan(request(&link)).await,
            Err(SecurityReportAdapterError::SymlinkPath(path)) if path == link
        ));

        write_spdx(&directory.path().join("valid.spdx.json"));
        let response = SecurityReportAdapter::new()
            .scan(request(directory.path()))
            .await
            .unwrap();
        assert_eq!(response.outcome.reports().len(), 1);
        assert!(
            response
                .outcome
                .limitations()
                .iter()
                .any(|value| value.contains("symlink"))
        );
    }

    #[tokio::test]
    async fn security_report_rejects_relative_escape_duplicate_and_missing_paths() {
        let directory = TestDirectory::new();
        let relative = SecurityReportRequest {
            generation: 1,
            paths: vec![PathBuf::from("../reports")],
        };
        assert!(matches!(
            SecurityReportAdapter::new().scan(relative).await,
            Err(SecurityReportAdapterError::InvalidRequest(_))
        ));
        let duplicate = SecurityReportRequest {
            generation: 1,
            paths: vec![directory.0.clone(), directory.0.clone()],
        };
        assert!(matches!(
            SecurityReportAdapter::new().scan(duplicate).await,
            Err(SecurityReportAdapterError::InvalidRequest(_))
        ));
        assert!(matches!(
            SecurityReportAdapter::new()
                .scan(request(&directory.path().join("stale.cve.json")))
                .await,
            Err(SecurityReportAdapterError::MissingPath(_))
        ));
    }

    #[tokio::test]
    async fn security_report_exposes_timeout_cancellation_and_worker_loss() {
        let directory = TestDirectory::new();
        write_cve(&directory.path().join("valid.cve.json"));
        assert_eq!(
            SecurityReportAdapter::new()
                .with_timeout(Duration::ZERO)
                .scan(request(directory.path()))
                .await,
            Err(SecurityReportAdapterError::Timeout(0))
        );

        let cancellation = SecurityReportCancellation::default();
        assert!(cancellation.cancel());
        assert_eq!(
            SecurityReportAdapter::new()
                .scan_with_cancellation(request(directory.path()), cancellation)
                .await,
            Err(SecurityReportAdapterError::Cancelled)
        );

        assert!(matches!(
            SecurityReportAdapter::new()
                .with_worker_panic()
                .scan(request(directory.path()))
                .await,
            Err(SecurityReportAdapterError::WorkerLost(_))
        ));
    }
}
