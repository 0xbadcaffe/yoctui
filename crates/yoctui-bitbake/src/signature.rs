use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    sync::Notify,
};
use yoctui_model::{
    DaemonCompatibilitySnapshot, MAX_SIGNATURE_DIFFERENCES, MAX_SIGNATURE_RECORDS,
    SignatureComparisonRequest, SignatureDifference, SignatureDifferenceCategory,
    SignatureIdentity, SignatureRecord, SignatureTarget, SignatureValue, compare_signature_records,
    normalize_signature_differences, normalize_signature_records,
};

use crate::BitBakeCommandPlanner;

const MAX_SIGNATURE_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_SIGNATURE_VARIABLES: usize = 4096;
const MAX_SIGNATURE_DEPENDENCIES: usize = 4096;
const MAX_SIGNATURE_LIMITATIONS: usize = 64;
const MAX_SIGNATURE_SCAN_ENTRIES: usize = 100_000;
const SIGNATURE_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
}

impl SignatureCommandSpec {
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureDumpResponse {
    pub target: SignatureTarget,
    pub records: Vec<SignatureRecord>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureComparisonResponse {
    pub request: SignatureComparisonRequest,
    pub differences: Vec<SignatureDifference>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SignatureAdapterError {
    #[error("invalid signature request: {0}")]
    InvalidRequest(String),
    #[error("signature build directory is unavailable: {0}")]
    BuildDirectory(PathBuf),
    #[error("signature path is missing")]
    MissingPath,
    #[error("signature path is outside the configured build directory: {0}")]
    PathEscape(PathBuf),
    #[error("signature path is not a regular file: {0}")]
    InvalidFile(PathBuf),
    #[error("signature tool is missing: {0}")]
    MissingTool(PathBuf),
    #[error("could not start signature tool: {0}")]
    Spawn(String),
    #[error("signature tool exited with {exit_code:?}: {message}")]
    NonZero {
        exit_code: Option<i32>,
        message: String,
    },
    #[error("signature tool output exceeded the {0} byte limit")]
    OutputLimit(usize),
    #[error("signature tool timed out after {0} seconds")]
    Timeout(u64),
    #[error("signature operation was cancelled")]
    Cancelled,
    #[error("signature data is malformed: {0}")]
    Malformed(String),
    #[error("signature I/O failed: {0}")]
    Io(String),
    #[error(transparent)]
    Authorization(#[from] crate::BitBakeCommandAuthorizationError),
}

#[derive(Debug, Clone, Default)]
pub struct SignatureCancellation {
    inner: Arc<SignatureCancellationInner>,
}

#[derive(Debug, Default)]
struct SignatureCancellationInner {
    requested: AtomicBool,
    notify: Notify,
}

impl SignatureCancellation {
    pub fn cancel(&self) -> bool {
        let first = !self.inner.requested.swap(true, Ordering::SeqCst);
        if first {
            self.inner.notify.notify_waiters();
        }
        first
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.requested.load(Ordering::SeqCst)
    }

    async fn cancelled(&self) {
        loop {
            if self.is_cancelled() {
                return;
            }
            let notified = self.inner.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

#[derive(Debug, Clone)]
pub struct SignatureAdapter {
    build_dir: PathBuf,
    dumpsig_program: PathBuf,
    diffsigs_program: PathBuf,
    timeout: Duration,
    compatibility: Option<DaemonCompatibilitySnapshot>,
}

impl SignatureAdapter {
    pub fn new(build_dir: PathBuf) -> Self {
        Self::with_programs(
            build_dir,
            PathBuf::from("bitbake-dumpsig"),
            PathBuf::from("bitbake-diffsigs"),
        )
    }

    pub fn with_programs(
        build_dir: PathBuf,
        dumpsig_program: PathBuf,
        diffsigs_program: PathBuf,
    ) -> Self {
        Self {
            build_dir,
            dumpsig_program,
            diffsigs_program,
            timeout: SIGNATURE_COMMAND_TIMEOUT,
            compatibility: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_compatibility(
        mut self,
        compatibility: DaemonCompatibilitySnapshot,
    ) -> Result<Self, SignatureAdapterError> {
        self.compatibility = Some(
            compatibility
                .normalize()
                .map_err(|error| SignatureAdapterError::InvalidRequest(error.to_string()))?,
        );
        Ok(self)
    }

    fn command_planner(
        &self,
        canonical_build_dir: &Path,
    ) -> Result<BitBakeCommandPlanner<'_>, SignatureAdapterError> {
        let compatibility = self.compatibility.as_ref().ok_or_else(|| {
            SignatureAdapterError::InvalidRequest(
                "signature commands require a daemon capability snapshot".into(),
            )
        })?;
        Ok(BitBakeCommandPlanner::new(
            compatibility,
            compatibility.snapshot.generation,
            canonical_build_dir,
        )?)
    }

    pub async fn dump(
        &self,
        target: SignatureTarget,
    ) -> Result<SignatureDumpResponse, SignatureAdapterError> {
        self.dump_with_cancellation(target, SignatureCancellation::default())
            .await
    }

    pub async fn dump_with_cancellation(
        &self,
        target: SignatureTarget,
        cancellation: SignatureCancellation,
    ) -> Result<SignatureDumpResponse, SignatureAdapterError> {
        target
            .validate()
            .map_err(|message| SignatureAdapterError::InvalidRequest(message.into()))?;
        let canonical_build_dir = canonical_build_dir(&self.build_dir).await?;
        let scan_root = canonical_build_dir.join("tmp/stamps");
        let target_for_scan = target.clone();
        let (paths, scan_truncated) = tokio::task::spawn_blocking(move || {
            discover_signature_paths(&scan_root, &target_for_scan)
        })
        .await
        .map_err(|error| SignatureAdapterError::Io(error.to_string()))??;

        let mut records = Vec::new();
        let mut limitations = Vec::new();
        if scan_truncated {
            push_limitation(
                &mut limitations,
                format!(
                    "signature scan stopped after {MAX_SIGNATURE_SCAN_ENTRIES} filesystem entries"
                ),
            );
        }
        for path in paths {
            if records.len() >= MAX_SIGNATURE_RECORDS {
                push_limitation(
                    &mut limitations,
                    format!("signature records were limited to {MAX_SIGNATURE_RECORDS} entries"),
                );
                break;
            }
            let path = validate_signature_path(&canonical_build_dir, &path).await?;
            let identity = identity_from_path(&target, path.clone())?;
            let authorized = self
                .command_planner(&canonical_build_dir)?
                .signature_dump(&path)?;
            let output = run_signature_command(
                SignatureCommandSpec {
                    executable: self
                        .authorized_signature_executable(&authorized, &self.dumpsig_program)?,
                    arguments: authorized.arguments,
                },
                &self.build_dir,
                self.timeout,
                &cancellation,
            )
            .await?;
            let (record, record_limitations) =
                parse_signature_dump(&identity, &String::from_utf8_lossy(&output))?;
            records.push(record);
            for limitation in record_limitations {
                push_limitation(&mut limitations, limitation);
            }
        }
        let (records, report) =
            normalize_signature_records(&target, records, MAX_SIGNATURE_RECORDS);
        if report.invalid_records > 0 {
            push_limitation(
                &mut limitations,
                format!(
                    "{} invalid signature record(s) were dropped",
                    report.invalid_records
                ),
            );
        }
        if report.truncated_records > 0 {
            push_limitation(
                &mut limitations,
                format!(
                    "{} signature record(s) exceeded the model limit",
                    report.truncated_records
                ),
            );
        }
        Ok(SignatureDumpResponse {
            target,
            records,
            limitations,
        })
    }

    pub async fn compare(
        &self,
        request: SignatureComparisonRequest,
    ) -> Result<SignatureComparisonResponse, SignatureAdapterError> {
        self.compare_with_cancellation(request, SignatureCancellation::default())
            .await
    }

    pub async fn compare_with_cancellation(
        &self,
        request: SignatureComparisonRequest,
        cancellation: SignatureCancellation,
    ) -> Result<SignatureComparisonResponse, SignatureAdapterError> {
        request
            .validate()
            .map_err(|message| SignatureAdapterError::InvalidRequest(message.into()))?;
        let canonical_build_dir = canonical_build_dir(&self.build_dir).await?;
        let left_path = validate_identity_path(&canonical_build_dir, &request.left).await?;
        let right_path = validate_identity_path(&canonical_build_dir, &request.right).await?;

        let planner = self.command_planner(&canonical_build_dir)?;
        let compare = planner.signature_compare(&left_path, &right_path)?;
        let dump_left = planner.signature_dump(&left_path)?;
        let dump_right = planner.signature_dump(&right_path)?;
        let diffsigs_output = run_signature_command(
            SignatureCommandSpec {
                executable: self
                    .authorized_signature_executable(&compare, &self.diffsigs_program)?,
                arguments: compare.arguments,
            },
            &self.build_dir,
            self.timeout,
            &cancellation,
        )
        .await?;
        let left_output = run_signature_command(
            SignatureCommandSpec {
                executable: self
                    .authorized_signature_executable(&dump_left, &self.dumpsig_program)?,
                arguments: dump_left.arguments,
            },
            &self.build_dir,
            self.timeout,
            &cancellation,
        )
        .await?;
        let right_output = run_signature_command(
            SignatureCommandSpec {
                executable: self
                    .authorized_signature_executable(&dump_right, &self.dumpsig_program)?,
                arguments: dump_right.arguments,
            },
            &self.build_dir,
            self.timeout,
            &cancellation,
        )
        .await?;

        let (left, mut limitations) =
            parse_signature_dump(&request.left, &String::from_utf8_lossy(&left_output))?;
        let (right, right_limitations) =
            parse_signature_dump(&request.right, &String::from_utf8_lossy(&right_output))?;
        for limitation in right_limitations {
            push_limitation(&mut limitations, limitation);
        }
        let (mut differences, report) =
            compare_signature_records(&left, &right, MAX_SIGNATURE_DIFFERENCES);
        let (tool_differences, tool_limitations) =
            parse_diffsigs_output(&String::from_utf8_lossy(&diffsigs_output));
        differences.extend(tool_differences);
        let (differences, combined_report) =
            normalize_signature_differences(differences, MAX_SIGNATURE_DIFFERENCES);
        if report.truncated_differences > 0 || combined_report.truncated_differences > 0 {
            push_limitation(
                &mut limitations,
                format!(
                    "signature differences were limited to {MAX_SIGNATURE_DIFFERENCES} entries"
                ),
            );
        }
        for limitation in tool_limitations {
            push_limitation(&mut limitations, limitation);
        }
        Ok(SignatureComparisonResponse {
            request,
            differences,
            limitations,
        })
    }

    fn authorized_signature_executable(
        &self,
        command: &crate::AuthorizedBitBakeCommand,
        configured: &Path,
    ) -> Result<PathBuf, SignatureAdapterError> {
        if command.executable != configured {
            return Err(SignatureAdapterError::InvalidRequest(format!(
                "configured signature executable {} does not match capability-authorized executable {}",
                configured.display(),
                command.executable.display()
            )));
        }
        Ok(command.executable.clone())
    }
}

async fn canonical_build_dir(build_dir: &Path) -> Result<PathBuf, SignatureAdapterError> {
    tokio::fs::canonicalize(build_dir)
        .await
        .map_err(|_| SignatureAdapterError::BuildDirectory(build_dir.to_owned()))
}

async fn validate_identity_path(
    canonical_build_dir: &Path,
    identity: &SignatureIdentity,
) -> Result<PathBuf, SignatureAdapterError> {
    identity
        .validate()
        .map_err(|message| SignatureAdapterError::InvalidRequest(message.into()))?;
    let path = identity
        .path
        .as_deref()
        .ok_or(SignatureAdapterError::MissingPath)?;
    let canonical = validate_signature_path(canonical_build_dir, path).await?;
    let discovered = identity_from_path(&identity.target, canonical.clone())?;
    if discovered.hash != identity.hash {
        return Err(SignatureAdapterError::InvalidRequest(format!(
            "signature hash does not match {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

async fn validate_signature_path(
    canonical_build_dir: &Path,
    path: &Path,
) -> Result<PathBuf, SignatureAdapterError> {
    if !path.is_absolute() {
        return Err(SignatureAdapterError::PathEscape(path.to_owned()));
    }
    let link_metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|_| SignatureAdapterError::InvalidFile(path.to_owned()))?;
    if link_metadata.file_type().is_symlink() || !link_metadata.is_file() {
        return Err(SignatureAdapterError::InvalidFile(path.to_owned()));
    }
    let canonical = tokio::fs::canonicalize(path)
        .await
        .map_err(|_| SignatureAdapterError::InvalidFile(path.to_owned()))?;
    if !canonical.starts_with(canonical_build_dir) {
        return Err(SignatureAdapterError::PathEscape(canonical));
    }
    Ok(canonical)
}

fn identity_from_path(
    target: &SignatureTarget,
    path: PathBuf,
) -> Result<SignatureIdentity, SignatureAdapterError> {
    let parent = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    if parent != Some(target.recipe.as_str()) {
        return Err(SignatureAdapterError::InvalidRequest(format!(
            "signature path does not belong to recipe {}: {}",
            target.recipe,
            path.display()
        )));
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| SignatureAdapterError::InvalidFile(path.clone()))?;
    let hash = signature_hash_from_name(name, &target.task).ok_or_else(|| {
        SignatureAdapterError::InvalidRequest(format!(
            "signature path does not match task {}: {}",
            target.task,
            path.display()
        ))
    })?;
    let identity = SignatureIdentity {
        target: target.clone(),
        hash,
        path: Some(path),
    };
    identity
        .validate()
        .map_err(|message| SignatureAdapterError::InvalidRequest(message.into()))?;
    Ok(identity)
}

fn signature_hash_from_name(name: &str, task: &str) -> Option<Option<String>> {
    for kind in ["sigdata", "siginfo"] {
        let marker = format!(".{task}.{kind}");
        let Some((_, suffix)) = name.split_once(&marker) else {
            continue;
        };
        if suffix.is_empty() {
            return Some(None);
        }
        let hash = suffix.strip_prefix('.')?;
        if hash.is_empty() || hash.contains('.') || hash.chars().any(char::is_whitespace) {
            return None;
        }
        return Some(Some(hash.to_owned()));
    }
    None
}

fn discover_signature_paths(
    root: &Path,
    target: &SignatureTarget,
) -> Result<(Vec<PathBuf>, bool), SignatureAdapterError> {
    if !root.is_dir() {
        return Ok((Vec::new(), false));
    }
    let mut directories = vec![root.to_owned()];
    let mut paths = Vec::new();
    let mut visited = 0usize;
    while let Some(directory) = directories.pop() {
        let mut entries = std::fs::read_dir(&directory)
            .map_err(|error| SignatureAdapterError::Io(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| SignatureAdapterError::Io(error.to_string()))?;
        entries.sort_by_key(std::fs::DirEntry::path);
        let mut child_directories = Vec::new();
        for entry in entries {
            visited += 1;
            if visited > MAX_SIGNATURE_SCAN_ENTRIES {
                paths.sort();
                return Ok((paths, true));
            }
            let file_type = entry
                .file_type()
                .map_err(|error| SignatureAdapterError::Io(error.to_string()))?;
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                child_directories.push(path);
            } else if file_type.is_file()
                && path
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    == Some(target.recipe.as_str())
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| signature_hash_from_name(name, &target.task).is_some())
            {
                paths.push(path);
            }
        }
        child_directories.reverse();
        directories.extend(child_directories);
    }
    paths.sort();
    Ok((paths, false))
}

struct BoundedOutput {
    bytes: Vec<u8>,
    truncated: bool,
}

async fn read_bounded<R>(mut reader: R) -> Result<BoundedOutput, std::io::Error>
where
    R: AsyncRead + Unpin,
{
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 16 * 1024];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let remaining = MAX_SIGNATURE_OUTPUT_BYTES.saturating_sub(bytes.len());
        bytes.extend_from_slice(&buffer[..read.min(remaining)]);
        truncated |= read > remaining;
    }
    Ok(BoundedOutput { bytes, truncated })
}

async fn run_signature_command(
    spec: SignatureCommandSpec,
    build_dir: &Path,
    timeout: Duration,
    cancellation: &SignatureCancellation,
) -> Result<Vec<u8>, SignatureAdapterError> {
    if cancellation.is_cancelled() {
        return Err(SignatureAdapterError::Cancelled);
    }
    let mut command = Command::new(&spec.executable);
    command
        .args(&spec.arguments)
        .current_dir(build_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            SignatureAdapterError::MissingTool(spec.executable.clone())
        } else {
            SignatureAdapterError::Spawn(error.to_string())
        }
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| SignatureAdapterError::Spawn("stdout is unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| SignatureAdapterError::Spawn("stderr is unavailable".into()))?;
    let stdout_task = tokio::spawn(read_bounded(stdout));
    let stderr_task = tokio::spawn(read_bounded(stderr));
    let terminal = tokio::select! {
        status = child.wait() => status.map_err(|error| SignatureAdapterError::Io(error.to_string())),
        _ = cancellation.cancelled() => {
            terminate_signature_child(&mut child).await;
            Err(SignatureAdapterError::Cancelled)
        }
        _ = tokio::time::sleep(timeout) => {
            terminate_signature_child(&mut child).await;
            Err(SignatureAdapterError::Timeout(timeout.as_secs()))
        }
    };
    let stdout = stdout_task
        .await
        .map_err(|error| SignatureAdapterError::Io(error.to_string()))?
        .map_err(|error| SignatureAdapterError::Io(error.to_string()))?;
    let stderr = stderr_task
        .await
        .map_err(|error| SignatureAdapterError::Io(error.to_string()))?
        .map_err(|error| SignatureAdapterError::Io(error.to_string()))?;
    let status = terminal?;
    if stdout.truncated || stderr.truncated {
        return Err(SignatureAdapterError::OutputLimit(
            MAX_SIGNATURE_OUTPUT_BYTES,
        ));
    }
    if !status.success() {
        return Err(SignatureAdapterError::NonZero {
            exit_code: status.code(),
            message: bounded_error_message(&stderr.bytes, &stdout.bytes),
        });
    }
    Ok(stdout.bytes)
}

async fn terminate_signature_child(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    if let Some(id) = child.id() {
        // SAFETY: the process group is the child PID created by `process_group(0)`.
        let _ = unsafe { libc::kill(-(id as i32), libc::SIGTERM) };
        if tokio::time::timeout(Duration::from_millis(500), child.wait())
            .await
            .is_ok()
        {
            return;
        }
        // SAFETY: same child-owned process group as the graceful signal above.
        let _ = unsafe { libc::kill(-(id as i32), libc::SIGKILL) };
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

fn bounded_error_message(stderr: &[u8], stdout: &[u8]) -> String {
    let bytes = if stderr.is_empty() { stdout } else { stderr };
    let text = String::from_utf8_lossy(bytes);
    let message = text.lines().next().unwrap_or("no diagnostic output");
    message.chars().take(512).collect()
}

fn parse_signature_dump(
    identity: &SignatureIdentity,
    output: &str,
) -> Result<(SignatureRecord, Vec<String>), SignatureAdapterError> {
    let mut base_hash = None;
    let mut task_hash = None;
    let mut variables = Vec::new();
    let mut task_dependencies = Vec::new();
    let mut dependency_hashes = BTreeMap::new();
    let mut limitations = Vec::new();
    let mut current_variable: Option<(String, String)> = None;
    let mut recognized = 0usize;

    for line in output.lines() {
        if let Some(value) = line.strip_prefix("basehash: ") {
            finish_variable(&mut current_variable, &mut variables, &mut limitations);
            base_hash = valid_hash(value).then(|| value.to_owned());
            recognized += 1;
        } else if let Some(value) = line.strip_prefix("Computed task hash is ") {
            finish_variable(&mut current_variable, &mut variables, &mut limitations);
            task_hash = valid_hash(value).then(|| value.to_owned());
            recognized += 1;
        } else if let Some(rest) = line.strip_prefix("Variable ") {
            if let Some((name, value)) = rest.split_once(" value is ") {
                finish_variable(&mut current_variable, &mut variables, &mut limitations);
                if valid_field_name(name) {
                    current_variable = Some((name.to_owned(), value.to_owned()));
                } else {
                    push_limitation(
                        &mut limitations,
                        "an invalid signature variable name was dropped".into(),
                    );
                }
                recognized += 1;
            } else if let Some((_, value)) = current_variable.as_mut() {
                append_variable_line(value, line, &mut limitations);
            }
        } else if let Some(value) = line.strip_prefix("Tasks this task depends on: ") {
            finish_variable(&mut current_variable, &mut variables, &mut limitations);
            match parse_quoted_list(value) {
                Some(dependencies) => task_dependencies = dependencies,
                None => push_limitation(
                    &mut limitations,
                    "task dependency list could not be parsed".into(),
                ),
            }
            recognized += 1;
        } else if let Some(value) = line.strip_prefix("Hash for dependent task ") {
            finish_variable(&mut current_variable, &mut variables, &mut limitations);
            if let Some((dependency, hash)) = value.rsplit_once(" is ")
                && valid_field_name(dependency)
                && valid_hash(hash)
            {
                dependency_hashes.insert(dependency.to_owned(), hash.to_owned());
            } else {
                push_limitation(
                    &mut limitations,
                    "a dependent task hash could not be parsed".into(),
                );
            }
            recognized += 1;
        } else if is_dump_header(line) {
            finish_variable(&mut current_variable, &mut variables, &mut limitations);
            recognized += 1;
        } else if let Some((_, value)) = current_variable.as_mut() {
            append_variable_line(value, line, &mut limitations);
        } else if !line.trim().is_empty() {
            push_limitation(
                &mut limitations,
                "unrecognized bitbake-dumpsig output was omitted".into(),
            );
        }
    }
    finish_variable(&mut current_variable, &mut variables, &mut limitations);
    if recognized == 0 {
        return Err(SignatureAdapterError::Malformed(
            "no recognized bitbake-dumpsig records".into(),
        ));
    }
    if variables.len() > MAX_SIGNATURE_VARIABLES {
        variables.sort();
        variables.truncate(MAX_SIGNATURE_VARIABLES);
        push_limitation(
            &mut limitations,
            format!("signature variables were limited to {MAX_SIGNATURE_VARIABLES} entries"),
        );
    }
    let mut dependencies = task_dependencies
        .into_iter()
        .map(|dependency| {
            dependency_hashes
                .get(&dependency)
                .map_or(dependency.clone(), |hash| format!("{dependency}={hash}"))
        })
        .collect::<Vec<_>>();
    for (dependency, hash) in dependency_hashes {
        if !dependencies
            .iter()
            .any(|value| value == &dependency || value.starts_with(&format!("{dependency}=")))
        {
            dependencies.push(format!("{dependency}={hash}"));
        }
    }
    dependencies.sort();
    dependencies.dedup();
    if dependencies.len() > MAX_SIGNATURE_DEPENDENCIES {
        dependencies.truncate(MAX_SIGNATURE_DEPENDENCIES);
        push_limitation(
            &mut limitations,
            format!("signature dependencies were limited to {MAX_SIGNATURE_DEPENDENCIES} entries"),
        );
    }
    if identity.hash.is_some() && task_hash != identity.hash {
        push_limitation(
            &mut limitations,
            "computed task hash did not match the signature filename".into(),
        );
    }
    Ok((
        SignatureRecord {
            identity: identity.clone(),
            base_hash,
            task_hash,
            variables,
            dependencies,
        },
        limitations,
    ))
}

fn is_dump_header(line: &str) -> bool {
    line.starts_with("basehash_ignore_vars:")
        || line.starts_with("taskhash_ignore_tasks:")
        || line.starts_with("Task dependencies:")
        || line.starts_with("List of dependencies for variable ")
        || line.starts_with("This task depends on the checksums of files:")
        || line.starts_with("Computed base hash is ")
        || line == "Unable to compute base hash"
        || line == "Unable to compute task hash"
        || line.starts_with("Tainted (by forced/invalidated task):")
}

fn finish_variable(
    current: &mut Option<(String, String)>,
    variables: &mut Vec<SignatureValue>,
    limitations: &mut Vec<String>,
) {
    let Some((name, mut value)) = current.take() else {
        return;
    };
    if value.len() > MAX_SIGNATURE_OUTPUT_BYTES {
        value.truncate(MAX_SIGNATURE_OUTPUT_BYTES);
        push_limitation(
            limitations,
            format!("signature variable {name} was truncated"),
        );
    }
    variables.push(SignatureValue {
        name,
        value: Some(value),
    });
}

fn append_variable_line(value: &mut String, line: &str, limitations: &mut Vec<String>) {
    if value.len() >= MAX_SIGNATURE_OUTPUT_BYTES {
        push_limitation(
            limitations,
            "a multiline signature variable was truncated".into(),
        );
        return;
    }
    value.push('\n');
    let remaining = MAX_SIGNATURE_OUTPUT_BYTES.saturating_sub(value.len());
    if line.len() <= remaining {
        value.push_str(line);
    } else {
        let boundary = line
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(line.len()))
            .take_while(|index| *index <= remaining)
            .last()
            .unwrap_or(0);
        value.push_str(&line[..boundary]);
    }
}

fn valid_hash(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.chars().any(char::is_whitespace)
        && !value.chars().any(char::is_control)
}

fn valid_field_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && !value.chars().any(char::is_control)
        && !value.contains('\n')
}

fn parse_quoted_list(value: &str) -> Option<Vec<String>> {
    let value = value.trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return None;
    }
    let mut values = Vec::new();
    let mut chars = value[1..value.len() - 1].chars().peekable();
    while let Some(character) = chars.next() {
        if character.is_whitespace() || character == ',' {
            continue;
        }
        if character != '\'' {
            return None;
        }
        let mut item = String::new();
        loop {
            match chars.next()? {
                '\\' => item.push(chars.next()?),
                '\'' => break,
                character if character.is_control() => return None,
                character => item.push(character),
            }
        }
        if !valid_field_name(&item) {
            return None;
        }
        values.push(item);
        if values.len() > MAX_SIGNATURE_DEPENDENCIES {
            break;
        }
    }
    Some(values)
}

fn parse_diffsigs_output(output: &str) -> (Vec<SignatureDifference>, Vec<String>) {
    let mut differences = Vec::new();
    let mut limitations = Vec::new();
    let mut represented_lines = BTreeSet::new();
    for (index, line) in output.lines().enumerate() {
        if let Some(value) = line.strip_prefix("basehash changed from ")
            && let Some((left, right)) = value.split_once(" to ")
        {
            differences.push(SignatureDifference {
                category: SignatureDifferenceCategory::BaseHash,
                key: "base_hash".into(),
                left: Some(left.to_owned()),
                right: Some(right.to_owned()),
            });
            represented_lines.insert(index);
        } else if let Some(value) = line.strip_prefix("Variable ")
            && let Some((name, values)) = value.split_once(" value changed from '")
            && let Some((left, right)) = values.split_once("' to '")
        {
            differences.push(SignatureDifference {
                category: SignatureDifferenceCategory::ChangedValue,
                key: name.to_owned(),
                left: Some(left.to_owned()),
                right: Some(right.trim_end_matches('\'').to_owned()),
            });
            represented_lines.insert(index);
        } else if let Some(name) = line
            .strip_prefix("Dependency on variable ")
            .and_then(|value| value.strip_suffix(" was added"))
        {
            differences.push(SignatureDifference {
                category: SignatureDifferenceCategory::Dependency,
                key: name.to_owned(),
                left: None,
                right: Some("present".into()),
            });
            represented_lines.insert(index);
        } else if let Some(name) = line
            .strip_prefix("Dependency on Variable ")
            .and_then(|value| value.strip_suffix(" was removed"))
        {
            differences.push(SignatureDifference {
                category: SignatureDifferenceCategory::Dependency,
                key: name.to_owned(),
                left: Some("present".into()),
                right: None,
            });
            represented_lines.insert(index);
        }
    }
    if output
        .lines()
        .enumerate()
        .any(|(index, line)| !line.trim().is_empty() && !represented_lines.contains(&index))
    {
        push_limitation(
            &mut limitations,
            "some recursive bitbake-diffsigs details were omitted; exact top-level dumps remain available"
                .into(),
        );
    }
    (differences, limitations)
}

fn push_limitation(limitations: &mut Vec<String>, limitation: String) {
    if limitations.len() < MAX_SIGNATURE_LIMITATIONS && !limitations.contains(&limitation) {
        limitations.push(limitation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    #[cfg(unix)]
    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "yoctui-signature-{name}-{}-{nonce}",
                std::process::id()
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

    fn target() -> SignatureTarget {
        SignatureTarget {
            recipe: "busybox".into(),
            task: "do_compile".into(),
        }
    }

    fn fixture(hash: &str) -> String {
        format!(
            "basehash_ignore_vars: []\n\
             taskhash_ignore_tasks: []\n\
             Task dependencies: ['CC']\n\
             basehash: base-{hash}\n\
             List of dependencies for variable CC is []\n\
             Variable CC value is gcc\n\
             Variable SCRIPT value is line one\n\
             line two\n\
             Tasks this task depends on: ['busybox:do_configure']\n\
             Hash for dependent task busybox:do_configure is dep-{hash}\n\
             Computed base hash is base-{hash} and from file base-{hash}\n\
             Computed task hash is {hash}\n"
        )
    }

    fn write_executable(path: &Path, body: &str) {
        crate::test_support::write_executable(path, body);
    }

    fn signature_path(root: &Path, hash: &str) -> PathBuf {
        let directory = root.join("tmp/stamps/qemux86_64/busybox");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("1.0.do_compile.sigdata.{hash}"));
        fs::write(&path, "{}").unwrap();
        path
    }

    fn test_compatibility(root: &Path, dump: &Path, diff: &Path) -> DaemonCompatibilitySnapshot {
        use yoctui_model::{
            AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind,
            CapabilityEvidenceOutcome, CapabilityId, CapabilityImplementation,
            CapabilityImplementationKind, CapabilityRecord, CapabilitySnapshot, CapabilityState,
            IdentityAuthority, ToolIdentity, YoctoEnvironmentIdentity,
        };
        let capabilities = [
            (
                CapabilityId::BitBakeDumpSig,
                crate::compatibility_command::BITBAKE_DUMPSIG_ARGV_IMPLEMENTATION,
            ),
            (
                CapabilityId::BitBakeDiffSigs,
                crate::compatibility_command::BITBAKE_DIFFSIGS_ARGV_IMPLEMENTATION,
            ),
        ];
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation: 1,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        root.to_owned(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    available_tools: AuthoritativeValue::detected(
                        vec![
                            ToolIdentity {
                                id: "bitbake-dumpsig".into(),
                                executable: dump.to_owned(),
                                version: None,
                            },
                            ToolIdentity {
                                id: "bitbake-diffsigs".into(),
                                executable: diff.to_owned(),
                                version: None,
                            },
                        ],
                        IdentityAuthority::ExecutableProbe,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities: capabilities
                    .iter()
                    .map(|(id, _)| CapabilityRecord {
                        id: *id,
                        state: CapabilityState::Available,
                        evidence: vec![CapabilityEvidence {
                            kind: CapabilityEvidenceKind::DirectProbe,
                            outcome: CapabilityEvidenceOutcome::Positive,
                            subject: format!("{} fixture probe", id.as_str()),
                            detail: "The exact signature helper argv is supported.".into(),
                            argv: Vec::new(),
                        }],
                    })
                    .collect(),
            },
            implementations: capabilities
                .into_iter()
                .map(|(id, implementation)| {
                    (
                        id,
                        CapabilityImplementation {
                            id: implementation.into(),
                            kind: CapabilityImplementationKind::Command,
                        },
                    )
                })
                .collect(),
        }
        .normalize()
        .unwrap()
    }

    fn test_adapter(root: &Path, dump: PathBuf, diff: PathBuf) -> SignatureAdapter {
        let compatibility = test_compatibility(root, &dump, &diff);
        SignatureAdapter::with_programs(root.to_owned(), dump, diff)
            .with_compatibility(compatibility)
            .unwrap()
    }

    #[test]
    fn signature_adapter_parser_keeps_typed_bounded_dump_data() {
        let identity = SignatureIdentity {
            target: target(),
            hash: Some("aaa".into()),
            path: Some("/build/tmp/stamps/qemux86_64/busybox/1.0.do_compile.sigdata.aaa".into()),
        };
        let (record, limitations) = parse_signature_dump(&identity, &fixture("aaa")).unwrap();
        assert!(limitations.is_empty());
        assert_eq!(record.base_hash.as_deref(), Some("base-aaa"));
        assert_eq!(record.task_hash.as_deref(), Some("aaa"));
        assert_eq!(record.variables.len(), 2);
        assert_eq!(
            record.variables[1].value.as_deref(),
            Some("line one\nline two")
        );
        assert_eq!(record.dependencies, vec!["busybox:do_configure=dep-aaa"]);

        assert!(matches!(
            parse_signature_dump(&identity, "nonsense"),
            Err(SignatureAdapterError::Malformed(_))
        ));
    }

    #[test]
    fn signature_adapter_parses_typed_diffsigs_summary_honestly() {
        let (differences, limitations) = parse_diffsigs_output(
            "basehash changed from old to new\n\
             Variable CC value changed from 'gcc' to 'clang'\n\
             Dependency on variable CFLAGS was added\n\
             recursive detail",
        );
        assert_eq!(differences.len(), 3);
        assert_eq!(limitations.len(), 1);
    }

    #[tokio::test]
    async fn signature_adapter_discovers_dumps_and_constructs_exact_arguments() {
        let directory = TestDirectory::new("dump");
        let path = signature_path(directory.path(), "aaa");
        let dump = directory.path().join("dump");
        let diff = directory.path().join("diff");
        write_executable(
            &dump,
            &format!(
                "#!/bin/sh\n[ \"$#\" -eq 1 ] || exit 8\n[ \"$1\" = \"{}\" ] || exit 9\nprintf '%s' '{}'\n",
                path.display(),
                fixture("aaa").replace('\'', "'\\''")
            ),
        );
        write_executable(&diff, "#!/bin/sh\nexit 0\n");
        let response = test_adapter(directory.path(), dump, diff)
            .dump(target())
            .await
            .unwrap();
        assert_eq!(response.records.len(), 1);
        assert_eq!(response.records[0].identity.path.as_ref(), Some(&path));
        assert!(response.limitations.is_empty());
    }

    #[tokio::test]
    async fn signature_adapter_compares_exact_validated_paths() {
        let directory = TestDirectory::new("compare");
        let left_path = signature_path(directory.path(), "aaa");
        let right_path = signature_path(directory.path(), "bbb");
        let dump = directory.path().join("dump");
        let diff = directory.path().join("diff");
        write_executable(
            &dump,
            &format!(
                "#!/bin/sh\ncase \"$1\" in\n*aaa) printf '%s' '{}';;\n*bbb) printf '%s' '{}';;\n*) exit 9;;\nesac\n",
                fixture("aaa").replace('\'', "'\\''"),
                fixture("bbb")
                    .replace("Variable CC value is gcc", "Variable CC value is clang")
                    .replace('\'', "'\\''")
            ),
        );
        write_executable(
            &diff,
            &format!(
                "#!/bin/sh\n[ \"$1\" = '-c' ] || exit 8\n[ \"$2\" = 'never' ] || exit 9\n[ \"$3\" = '{}' ] || exit 10\n[ \"$4\" = '{}' ] || exit 11\nprintf '%s\\n' \"basehash changed from base-aaa to base-bbb\"\n",
                left_path.display(),
                right_path.display()
            ),
        );
        let request = SignatureComparisonRequest {
            left: identity_from_path(&target(), left_path).unwrap(),
            right: identity_from_path(&target(), right_path).unwrap(),
        };
        let response = test_adapter(directory.path(), dump, diff)
            .compare(request.clone())
            .await
            .unwrap();
        assert_eq!(response.request, request);
        assert!(response.limitations.is_empty());
        assert!(response.differences.iter().any(|difference| {
            difference.category == SignatureDifferenceCategory::ChangedValue
                && difference.key == "CC"
        }));
    }

    #[tokio::test]
    async fn signature_adapter_rejects_escape_missing_tools_and_nonzero_results() {
        let directory = TestDirectory::new("errors");
        let outside_directory = TestDirectory::new("outside");
        let outside = outside_directory.path().join("outside.sigdata.aaa");
        fs::write(&outside, "{}").unwrap();
        let identity = SignatureIdentity {
            target: target(),
            hash: Some("aaa".into()),
            path: Some(outside.clone()),
        };
        let request = SignatureComparisonRequest {
            left: identity,
            right: SignatureIdentity {
                target: target(),
                hash: Some("bbb".into()),
                path: Some(outside),
            },
        };
        let adapter = test_adapter(
            directory.path(),
            directory.path().join("missing"),
            directory.path().join("missing"),
        );
        assert!(matches!(
            adapter.compare(request).await,
            Err(SignatureAdapterError::PathEscape(_))
        ));

        let path = signature_path(directory.path(), "aaa");
        assert!(matches!(
            adapter.dump(target()).await,
            Err(SignatureAdapterError::MissingTool(_))
        ));
        let failure = directory.path().join("failure");
        write_executable(&failure, "#!/bin/sh\nprintf 'bad input\\n' >&2\nexit 7\n");
        let error = test_adapter(directory.path(), failure, directory.path().join("unused"))
            .dump(target())
            .await
            .unwrap_err();
        assert_eq!(
            error,
            SignatureAdapterError::NonZero {
                exit_code: Some(7),
                message: "bad input".into()
            }
        );
        assert!(path.exists());
    }

    #[tokio::test]
    async fn signature_adapter_bounds_output_and_supports_cancellation() {
        let directory = TestDirectory::new("bounds");
        signature_path(directory.path(), "aaa");
        let oversized = directory.path().join("oversized");
        write_executable(
            &oversized,
            "#!/bin/sh\nhead -c 9000000 /dev/zero | tr '\\0' x\n",
        );
        let error = test_adapter(directory.path(), oversized, directory.path().join("unused"))
            .dump(target())
            .await
            .unwrap_err();
        assert_eq!(
            error,
            SignatureAdapterError::OutputLimit(MAX_SIGNATURE_OUTPUT_BYTES)
        );

        let sleeping = directory.path().join("sleeping");
        write_executable(&sleeping, "#!/bin/sh\nsleep 30\n");
        let adapter = test_adapter(directory.path(), sleeping, directory.path().join("unused"))
            .with_timeout(Duration::from_secs(60));
        let cancellation = SignatureCancellation::default();
        let cancel_handle = cancellation.clone();
        let operation =
            tokio::spawn(
                async move { adapter.dump_with_cancellation(target(), cancellation).await },
            );
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(cancel_handle.cancel());
        assert!(!cancel_handle.cancel());
        assert_eq!(
            operation.await.unwrap().unwrap_err(),
            SignatureAdapterError::Cancelled
        );
    }

    #[tokio::test]
    async fn signature_adapter_reports_empty_malformed_and_duplicate_results() {
        let directory = TestDirectory::new("malformed");
        let dump = directory.path().join("dump");
        let diff = directory.path().join("diff");
        write_executable(&dump, "#!/bin/sh\nprintf 'nonsense\\n'\n");
        write_executable(&diff, "#!/bin/sh\nexit 0\n");
        let adapter = test_adapter(directory.path(), dump.clone(), diff);
        assert!(adapter.dump(target()).await.unwrap().records.is_empty());

        signature_path(directory.path(), "aaa");
        assert!(matches!(
            adapter.dump(target()).await,
            Err(SignatureAdapterError::Malformed(_))
        ));

        let identity = SignatureIdentity {
            target: target(),
            hash: Some("aaa".into()),
            path: Some(
                directory
                    .path()
                    .join("tmp/stamps/qemux86_64/busybox/1.0.do_compile.sigdata.aaa"),
            ),
        };
        let (left, _) = parse_signature_dump(&identity, &fixture("aaa")).unwrap();
        let (records, report) = normalize_signature_records(&target(), vec![left.clone(), left], 8);
        assert_eq!(records.len(), 1);
        assert_eq!(report.duplicate_records, 1);
    }
}
