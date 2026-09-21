#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDataCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
}

impl PackageDataCommandSpec {
    pub(crate) fn new(
        executable: &Path,
        pkgdata_dir: &Path,
        subcommand: &str,
        arguments: impl IntoIterator<Item = OsString>,
    ) -> Self {
        let mut exact = vec![
            OsString::from("-p"),
            pkgdata_dir.as_os_str().to_owned(),
            OsString::from(subcommand),
        ];
        exact.extend(arguments);
        Self {
            executable: executable.to_owned(),
            arguments: exact,
        }
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInventoryResponse {
    pub request: PackageInventoryRequest,
    pub packages: Vec<PackageSummary>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDetailResponse {
    pub request: PackageDetailRequest,
    pub detail: PackageDetail,
    pub limitations: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PackageDataAdapterError {
    #[error("invalid package-data request: {0}")]
    InvalidRequest(String),
    #[error("package-data build directory is unavailable: {0}")]
    BuildDirectory(PathBuf),
    #[error("generated pkgdata is unavailable: {0}; build a target through do_package first")]
    MissingPkgdata(PathBuf),
    #[error("package-data path is invalid or is a symlink: {0}")]
    InvalidPath(PathBuf),
    #[error("package-data path escapes its configured root: {0}")]
    PathEscape(PathBuf),
    #[error("oe-pkgdata-util is missing beneath: {0}")]
    MissingTool(PathBuf),
    #[error("package-data capability {capability:?} is unavailable: {reason}")]
    CapabilityUnavailable {
        capability: CapabilityId,
        reason: String,
    },
    #[error("package-data capability generation is stale: expected {expected}, got {actual}")]
    StaleCapability { expected: u64, actual: u64 },
    #[error("package-data capability snapshot belongs to another build environment")]
    CapabilityEnvironmentMismatch,
    #[error("could not start oe-pkgdata-util: {0}")]
    Spawn(String),
    #[error("oe-pkgdata-util exited with {exit_code:?}: {message}")]
    NonZero {
        exit_code: Option<i32>,
        message: String,
    },
    #[error("oe-pkgdata-util timed out after {0} seconds")]
    Timeout(u64),
    #[error("package-data operation was cancelled")]
    Cancelled,
    #[error("package-data output is malformed: {0}")]
    Malformed(String),
    #[error("package-data I/O failed: {0}")]
    Io(String),
}

#[derive(Debug, Clone, Default)]
pub struct PackageDataCancellation {
    inner: Arc<PackageDataCancellationInner>,
}

#[derive(Debug, Default)]
struct PackageDataCancellationInner {
    requested: AtomicBool,
    notify: Notify,
}

impl PackageDataCancellation {
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
