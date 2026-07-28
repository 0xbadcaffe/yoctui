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

fn scan_deploy_root(
    request: SdkArtifactInventoryRequest,
    deploy_root: PathBuf,
    cancellation: SdkArtifactCancellation,
    deadline: Instant,
) -> Result<SdkArtifactResponse, SdkArtifactAdapterError> {
    validate_root(&deploy_root)?;
    let root =
        fs::canonicalize(&deploy_root).map_err(|error| root_io_error(&deploy_root, error))?;
    if root != deploy_root {
        return Err(SdkArtifactAdapterError::InvalidRoot(deploy_root));
    }
    check_scan_control(&cancellation, deadline)?;

    let mut limitations = Vec::new();
    let mut directories = BTreeSet::from([root.clone()]);
    let mut visited_directories = 0_usize;
    let mut files = Vec::new();
    let mut omitted_directories = 0_usize;
    let mut omitted_records = 0_usize;

    while let Some(directory) = directories.pop_first() {
        visited_directories = visited_directories.saturating_add(1);
        check_scan_control(&cancellation, deadline)?;
        let entries = bounded_directory_entries(
            &directory,
            directory == root,
            &cancellation,
            deadline,
            &mut limitations,
        )?;
        for path in entries {
            check_scan_control(&cancellation, deadline)?;
            if path.as_os_str().len() > MAX_SDK_PATH_BYTES {
                push_limitation(
                    &mut limitations,
                    format!("SDK entry exceeded the {MAX_SDK_PATH_BYTES}-byte path bound"),
                );
                continue;
            }
            if !valid_record_name(&path) {
                push_limitation(
                    &mut limitations,
                    "SDK entry had a malformed or oversized name".into(),
                );
                continue;
            }
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    push_limitation(
                        &mut limitations,
                        format!(
                            "metadata was unavailable for SDK entry {}: {error}",
                            path_label(&path)
                        ),
                    );
                    continue;
                }
            };
            if metadata.file_type().is_symlink() {
                push_limitation(
                    &mut limitations,
                    format!("SDK symlink was not followed: {}", path_label(&path)),
                );
                continue;
            }
            if metadata.is_dir() {
                if visited_directories.saturating_add(directories.len()) >= MAX_SDK_DIRECTORIES {
                    omitted_directories = omitted_directories.saturating_add(1);
                } else {
                    let canonical = canonical_descendant(&root, &path)?;
                    directories.insert(canonical);
                }
                continue;
            }
            if !metadata.is_file() {
                push_limitation(
                    &mut limitations,
                    format!("non-regular SDK entry was ignored: {}", path_label(&path)),
                );
                continue;
            }
            if files.len() >= MAX_SDK_ARTIFACTS {
                omitted_records = omitted_records.saturating_add(1);
                continue;
            }
            let canonical = canonical_descendant(&root, &path)?;
            let Some(kind) = classify_record(&canonical) else {
                push_limitation(
                    &mut limitations,
                    format!(
                        "malformed SDK record was ignored: {}",
                        path_label(&canonical)
                    ),
                );
                continue;
            };
            let Some(modified_unix_seconds) = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
            else {
                push_limitation(
                    &mut limitations,
                    format!(
                        "SDK record modification time was unavailable: {}",
                        path_label(&canonical)
                    ),
                );
                continue;
            };
            files.push(FileRecord {
                path: canonical,
                size_bytes: metadata.len(),
                modified_unix_seconds,
                kind,
            });
        }
    }

    if omitted_directories > 0 {
        push_limitation(
            &mut limitations,
            format!(
                "{omitted_directories} SDK directories were omitted at the {MAX_SDK_DIRECTORIES}-directory bound"
            ),
        );
    }
    if omitted_records > 0 {
        push_limitation(
            &mut limitations,
            format!(
                "{omitted_records} SDK records were omitted at the {MAX_SDK_ARTIFACTS}-record bound"
            ),
        );
    }

    files.sort_by(|left, right| left.path.cmp(&right.path));
    let mut artifacts = files
        .into_iter()
        .map(|record| SdkArtifact {
            identity: SdkArtifactIdentity {
                path: record.path,
                size_bytes: record.size_bytes,
                modified_unix_seconds: record.modified_unix_seconds,
            },
            kind: record.kind,
            sdk_kind: None,
            machine: None,
            host_tuple: None,
            target_tuple: None,
            checksums: Vec::new(),
            manifests: Vec::new(),
            published: None,
        })
        .collect::<Vec<_>>();
    associate_records(&mut artifacts, &mut limitations);
    let artifacts = normalize_sdk_artifacts(&request, artifacts)
        .map_err(|message| SdkArtifactAdapterError::Io(message.into()))?;
    let limitations = normalize_sdk_limitations(limitations);
    let outcome = if artifacts.is_empty() && limitations.is_empty() {
        SdkArtifactScanOutcome::Empty
    } else if limitations.is_empty() {
        SdkArtifactScanOutcome::Complete(artifacts)
    } else {
        SdkArtifactScanOutcome::Partial {
            artifacts,
            limitations,
        }
    };
    Ok(SdkArtifactResponse { request, outcome })
}

fn validate_root(root: &Path) -> Result<(), SdkArtifactAdapterError> {
    if !root.is_absolute() {
        return Err(SdkArtifactAdapterError::InvalidRoot(root.into()));
    }
    let metadata = fs::symlink_metadata(root).map_err(|error| root_io_error(root, error))?;
    if metadata.file_type().is_symlink() {
        return Err(SdkArtifactAdapterError::SymlinkRoot(root.into()));
    }
    if !metadata.is_dir() {
        return Err(SdkArtifactAdapterError::InvalidRoot(root.into()));
    }
    Ok(())
}

fn root_io_error(path: &Path, error: io::Error) -> SdkArtifactAdapterError {
    match error.kind() {
        io::ErrorKind::NotFound => SdkArtifactAdapterError::MissingRoot(path.into()),
        io::ErrorKind::PermissionDenied => SdkArtifactAdapterError::PermissionDenied(path.into()),
        _ => SdkArtifactAdapterError::Io(format!("{}: {error}", path.display())),
    }
}

fn bounded_directory_entries(
    directory: &Path,
    is_root: bool,
    cancellation: &SdkArtifactCancellation,
    deadline: Instant,
    limitations: &mut Vec<String>,
) -> Result<Vec<PathBuf>, SdkArtifactAdapterError> {
    let reader = match fs::read_dir(directory) {
        Ok(reader) => reader,
        Err(error) if is_root => return Err(root_io_error(directory, error)),
        Err(error) => {
            push_limitation(
                limitations,
                format!(
                    "SDK directory was unreadable and skipped: {}: {error}",
                    path_label(directory)
                ),
            );
            return Ok(Vec::new());
        }
    };
    let mut selected = BTreeSet::new();
    let mut omitted = 0_usize;
    for entry in reader {
        check_scan_control(cancellation, deadline)?;
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(error) => {
                push_limitation(
                    limitations,
                    format!("one SDK directory entry was unreadable: {error}"),
                );
                continue;
            }
        };
        selected.insert(path);
        if selected.len() > MAX_DIRECTORY_ENTRIES {
            selected.pop_last();
            omitted = omitted.saturating_add(1);
        }
    }
    if omitted > 0 {
        push_limitation(
            limitations,
            format!(
                "{omitted} SDK entries were omitted from {} at the {MAX_DIRECTORY_ENTRIES}-entry bound",
                path_label(directory)
            ),
        );
    }
    Ok(selected.into_iter().collect())
}

fn canonical_descendant(root: &Path, path: &Path) -> Result<PathBuf, SdkArtifactAdapterError> {
    let canonical =
        fs::canonicalize(path).map_err(|error| SdkArtifactAdapterError::Io(error.to_string()))?;
    if canonical != path || !canonical.starts_with(root) || canonical == root {
        return Err(SdkArtifactAdapterError::Io(format!(
            "SDK entry escaped or was not canonical: {}",
            path_label(path)
        )));
    }
    Ok(canonical)
}

fn check_scan_control(
    cancellation: &SdkArtifactCancellation,
    deadline: Instant,
) -> Result<(), SdkArtifactAdapterError> {
    if cancellation.is_cancelled() {
        Err(SdkArtifactAdapterError::Cancelled)
    } else if Instant::now() >= deadline {
        Err(SdkArtifactAdapterError::Timeout(0))
    } else {
        Ok(())
    }
}

fn valid_record_name(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            !name.is_empty()
                && name.len() <= MAX_SDK_NAME_BYTES
                && !matches!(name, "." | "..")
                && !name.chars().any(char::is_control)
        })
}

fn classify_record(path: &Path) -> Option<SdkArtifactKind> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    let suffix = checksum_suffix(&name);
    if let Some(suffix) = suffix {
        return (name.len() > suffix.len()).then_some(SdkArtifactKind::Checksum);
    }
    if name.ends_with(".manifest") {
        return (name.len() > ".manifest".len()).then_some(SdkArtifactKind::Manifest);
    }
    if name.ends_with(".sh") {
        return (name.len() > ".sh".len()).then_some(SdkArtifactKind::Installer);
    }
    Some(SdkArtifactKind::Other)
}

fn checksum_suffix(name: &str) -> Option<&'static str> {
    [".sha256sum", ".sha512", ".sha256", ".md5sum", ".md5"]
        .into_iter()
        .find(|suffix| name.ends_with(suffix))
}

fn associate_records(artifacts: &mut [SdkArtifact], limitations: &mut Vec<String>) {
    let checksums = artifacts
        .iter()
        .filter(|artifact| artifact.kind == SdkArtifactKind::Checksum)
        .map(|artifact| artifact.identity.path.clone())
        .collect::<Vec<_>>();
    let manifests = artifacts
        .iter()
        .filter(|artifact| artifact.kind == SdkArtifactKind::Manifest)
        .map(|artifact| artifact.identity.path.clone())
        .collect::<Vec<_>>();

    for artifact in artifacts
        .iter_mut()
        .filter(|artifact| artifact.kind == SdkArtifactKind::Installer)
    {
        let installer = &artifact.identity.path;
        artifact.checksums = associated_paths(
            installer,
            &checksums,
            SdkArtifactKind::Checksum,
            limitations,
        );
        artifact.manifests = associated_paths(
            installer,
            &manifests,
            SdkArtifactKind::Manifest,
            limitations,
        );
    }
}

fn associated_paths(
    installer: &Path,
    candidates: &[PathBuf],
    kind: SdkArtifactKind,
    limitations: &mut Vec<String>,
) -> Vec<PathBuf> {
    let installer_name = installer
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let installer_stem = installer
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let mut associated = candidates
        .iter()
        .filter(|candidate| candidate.parent() == installer.parent())
        .filter(|candidate| {
            let Some(name) = candidate.file_name().and_then(|name| name.to_str()) else {
                return false;
            };
            let base = match kind {
                SdkArtifactKind::Checksum => checksum_suffix(name)
                    .and_then(|suffix| name.strip_suffix(suffix))
                    .unwrap_or_default(),
                SdkArtifactKind::Manifest => manifest_base(name),
                _ => "",
            };
            base == installer_name || base == installer_stem
        })
        .cloned()
        .collect::<Vec<_>>();
    associated.sort();
    associated.dedup();
    if associated.len() > MAX_SDK_ASSOCIATIONS {
        let omitted = associated.len() - MAX_SDK_ASSOCIATIONS;
        associated.truncate(MAX_SDK_ASSOCIATIONS);
        push_limitation(
            limitations,
            format!(
                "{omitted} {:?} associations were omitted for {}",
                kind,
                path_label(installer)
            ),
        );
    }
    associated
}

fn manifest_base(name: &str) -> &str {
    for suffix in [".host.manifest", ".target.manifest", ".manifest"] {
        if let Some(base) = name.strip_suffix(suffix) {
            return base;
        }
    }
    ""
}

fn path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| name.len() <= MAX_SDK_NAME_BYTES && !name.chars().any(char::is_control))
        .unwrap_or("<unavailable>")
        .to_owned()
}

fn push_limitation(limitations: &mut Vec<String>, limitation: String) {
    if limitations.len() < yoctui_model::MAX_SDK_LIMITATIONS {
        limitations.push(limitation);
    } else if limitations.len() == yoctui_model::MAX_SDK_LIMITATIONS {
        limitations.pop();
        limitations.push("additional SDK scan limitations were omitted".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-sdk-artifact-{}-{}",
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

    fn fixture() -> TestDirectory {
        TestDirectory::new()
    }

    fn request(root: PathBuf) -> SdkArtifactInventoryRequest {
        SdkArtifactInventoryRequest {
            generation: 1,
            root,
            machine: "qemux86-64".into(),
        }
    }

    #[tokio::test]
    async fn sdk_artifact_scan_sorts_classifies_and_associates_records() {
        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("poky-toolchain.sh.target.manifest"), b"target").unwrap();
        fs::write(root.join("poky-toolchain.sh.sha256"), b"digest").unwrap();
        fs::write(root.join("poky-toolchain.host.manifest"), b"host").unwrap();
        fs::write(root.join("poky-toolchain.sh"), b"installer").unwrap();
        fs::write(root.join("README.txt"), b"other").unwrap();

        let response = SdkArtifactAdapter::new(root.clone())
            .scan(request(root))
            .await
            .unwrap();
        let SdkArtifactScanOutcome::Complete(artifacts) = response.outcome else {
            panic!("expected complete SDK inventory");
        };
        assert!(
            artifacts
                .windows(2)
                .all(|pair| pair[0].identity < pair[1].identity)
        );
        let installer = artifacts
            .iter()
            .find(|artifact| artifact.kind == SdkArtifactKind::Installer)
            .unwrap();
        assert_eq!(installer.checksums.len(), 1);
        assert_eq!(installer.manifests.len(), 2);
        assert!(installer.identity.size_bytes > 0);
    }

    #[tokio::test]
    async fn sdk_artifact_scan_distinguishes_empty_and_unavailable_metadata() {
        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        let empty = SdkArtifactAdapter::new(root.clone())
            .scan(request(root.clone()))
            .await
            .unwrap();
        assert_eq!(empty.outcome, SdkArtifactScanOutcome::Empty);

        fs::write(root.join("poky-toolchain.sh"), b"installer").unwrap();
        let response = SdkArtifactAdapter::new(root.clone())
            .scan(request(root))
            .await
            .unwrap();
        let artifact = response.outcome.artifacts().first().unwrap();
        assert_eq!(artifact.sdk_kind, None);
        assert_eq!(artifact.machine, None);
        assert_eq!(artifact.host_tuple, None);
        assert_eq!(artifact.target_tuple, None);
        assert_eq!(artifact.published, None);
    }

    #[tokio::test]
    async fn sdk_artifact_scan_reports_partial_malformed_and_oversized_records() {
        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("valid.sh"), b"installer").unwrap();
        fs::write(root.join(".manifest"), b"malformed").unwrap();
        let long_name = format!("{}.txt", "x".repeat(MAX_SDK_NAME_BYTES));
        File::create(root.join(long_name))
            .unwrap()
            .write_all(b"x")
            .unwrap();

        let response = SdkArtifactAdapter::new(root.clone())
            .scan(request(root))
            .await
            .unwrap();
        assert!(matches!(
            response.outcome,
            SdkArtifactScanOutcome::Partial { .. }
        ));
        assert!(
            response
                .outcome
                .limitations()
                .iter()
                .any(|message| { message.contains("malformed") || message.contains("oversized") })
        );
        assert_eq!(response.outcome.artifacts().len(), 1);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_artifact_scan_rejects_root_symlink_mismatch_and_entry_escape() {
        use std::os::unix::fs::symlink;

        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        let missing = fixture.path().join("missing");
        assert!(matches!(
            SdkArtifactAdapter::new(missing.clone())
                .scan(request(missing))
                .await,
            Err(SdkArtifactAdapterError::MissingRoot(_))
        ));

        let linked = fixture.path().join("linked");
        symlink(&root, &linked).unwrap();
        assert!(matches!(
            SdkArtifactAdapter::new(linked.clone())
                .scan(request(linked))
                .await,
            Err(SdkArtifactAdapterError::SymlinkRoot(_))
        ));

        let other = fixture.path().join("other");
        fs::create_dir(&other).unwrap();
        assert!(matches!(
            SdkArtifactAdapter::new(root.clone())
                .scan(request(other))
                .await,
            Err(SdkArtifactAdapterError::RootMismatch { .. })
        ));

        let outside = fixture.path().join("outside.sh");
        fs::write(&outside, b"outside").unwrap();
        symlink(&outside, root.join("escaped.sh")).unwrap();
        let response = SdkArtifactAdapter::new(root.clone())
            .scan(request(root))
            .await
            .unwrap();
        assert!(matches!(
            response.outcome,
            SdkArtifactScanOutcome::Partial { .. }
        ));
        assert!(
            response
                .outcome
                .limitations()
                .iter()
                .any(|message| message.contains("symlink"))
        );
    }

    #[tokio::test]
    async fn sdk_artifact_scan_has_distinct_timeout_cancellation_permission_and_worker_loss() {
        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        let cancellation = SdkArtifactCancellation::default();
        cancellation.cancel();
        assert_eq!(
            SdkArtifactAdapter::new(root.clone())
                .scan_with_cancellation(request(root.clone()), cancellation)
                .await,
            Err(SdkArtifactAdapterError::Cancelled)
        );
        assert!(matches!(
            SdkArtifactAdapter::new(root.clone())
                .with_timeout(Duration::ZERO)
                .scan(request(root.clone()))
                .await,
            Err(SdkArtifactAdapterError::Timeout(_))
        ));
        assert!(matches!(
            SdkArtifactAdapter::new(root.clone())
                .with_worker_panic()
                .scan(request(root.clone()))
                .await,
            Err(SdkArtifactAdapterError::WorkerLost(_))
        ));
        assert_eq!(
            root_io_error(
                &root,
                io::Error::new(io::ErrorKind::PermissionDenied, "denied")
            ),
            SdkArtifactAdapterError::PermissionDenied(root)
        );
    }

    #[tokio::test]
    async fn sdk_artifact_scan_bounds_directory_entries_deterministically() {
        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        for index in (0..=MAX_DIRECTORY_ENTRIES).rev() {
            fs::write(root.join(format!("{index:05}.txt")), b"record").unwrap();
        }

        let first = SdkArtifactAdapter::new(root.clone())
            .scan(request(root.clone()))
            .await
            .unwrap();
        let second = SdkArtifactAdapter::new(root.clone())
            .scan(request(root))
            .await
            .unwrap();
        assert!(matches!(
            first.outcome,
            SdkArtifactScanOutcome::Partial { .. }
        ));
        assert_eq!(first.outcome.artifacts(), second.outcome.artifacts());
        assert_eq!(first.outcome.limitations(), second.outcome.limitations());
        assert_eq!(first.outcome.artifacts().len(), MAX_DIRECTORY_ENTRIES);
        assert!(
            first
                .outcome
                .artifacts()
                .iter()
                .all(|artifact| !artifact.identity.path.ends_with("04096.txt"))
        );
    }

    #[tokio::test]
    async fn sdk_artifact_scan_bounds_traversed_directories() {
        let fixture = fixture();
        let root = fixture.path().join("sdk");
        fs::create_dir(&root).unwrap();
        let mut directory = root.clone();
        for index in 0..=MAX_SDK_DIRECTORIES {
            directory = directory.join(format!("{index:03}"));
            fs::create_dir(&directory).unwrap();
            fs::write(directory.join("record.txt"), b"record").unwrap();
        }

        let response = SdkArtifactAdapter::new(root.clone())
            .scan(request(root))
            .await
            .unwrap();
        assert!(matches!(
            response.outcome,
            SdkArtifactScanOutcome::Partial { .. }
        ));
        assert!(
            response
                .outcome
                .limitations()
                .iter()
                .any(|message| message.contains("directories were omitted"))
        );
        assert_eq!(response.outcome.artifacts().len(), MAX_SDK_DIRECTORIES - 1);
    }
}
