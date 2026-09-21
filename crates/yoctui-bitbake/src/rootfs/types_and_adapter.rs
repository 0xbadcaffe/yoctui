#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootfsCompositionSources {
    pub image: ImageArtifactIdentity,
    pub manifest: Option<PathBuf>,
    pub pkgdata_directory: Option<PathBuf>,
    pub image_rootfs: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootfsCompositionResponse {
    pub request: RootfsCompositionRequest,
    pub composition: RootfsComposition,
    pub limitations: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RootfsCompositionAdapterError {
    #[error("invalid rootfs composition request: {0}")]
    InvalidRequest(String),
    #[error("rootfs composition source belongs to another image")]
    ImageMismatch,
    #[error("rootfs composition generation is stale: expected {expected}, got {actual}")]
    StaleGeneration { expected: u64, actual: u64 },
    #[error("rootfs composition build directory is unavailable: {0}")]
    BuildDirectory(PathBuf),
    #[error("rootfs composition source is invalid or is a symlink: {0}")]
    InvalidSource(PathBuf),
    #[error("rootfs composition source escapes the active build: {0}")]
    PathEscape(PathBuf),
    #[error("rootfs composition scan timed out after {0} seconds")]
    Timeout(u64),
    #[error("rootfs composition scan was cancelled")]
    Cancelled,
    #[error("rootfs composition safety bound reached: {0}")]
    ResourceLimit(String),
    #[error("rootfs composition I/O failed: {0}")]
    Io(String),
}

#[derive(Debug, Clone, Default)]
pub struct RootfsCompositionCancellation {
    requested: Arc<AtomicBool>,
}

impl RootfsCompositionCancellation {
    pub fn cancel(&self) -> bool {
        !self.requested.swap(true, Ordering::SeqCst)
    }

    pub fn is_cancelled(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone)]
pub struct RootfsCompositionAdapter {
    build_directory: PathBuf,
    sources: RootfsCompositionSources,
    expected_generation: u64,
    timeout: Duration,
}

impl RootfsCompositionAdapter {
    pub fn new(
        build_directory: PathBuf,
        sources: RootfsCompositionSources,
        expected_generation: u64,
    ) -> Self {
        Self {
            build_directory,
            sources,
            expected_generation,
            timeout: ROOTFS_SCAN_TIMEOUT,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn scan(
        &self,
        request: RootfsCompositionRequest,
    ) -> Result<RootfsCompositionResponse, RootfsCompositionAdapterError> {
        self.scan_with_cancellation(request, RootfsCompositionCancellation::default())
            .await
    }

    pub async fn scan_with_cancellation(
        &self,
        request: RootfsCompositionRequest,
        cancellation: RootfsCompositionCancellation,
    ) -> Result<RootfsCompositionResponse, RootfsCompositionAdapterError> {
        request
            .validate()
            .map_err(|message| RootfsCompositionAdapterError::InvalidRequest(message.into()))?;
        if request.generation != self.expected_generation {
            return Err(RootfsCompositionAdapterError::StaleGeneration {
                expected: self.expected_generation,
                actual: request.generation,
            });
        }
        if request.image != self.sources.image {
            return Err(RootfsCompositionAdapterError::ImageMismatch);
        }
        if cancellation.is_cancelled() {
            return Err(RootfsCompositionAdapterError::Cancelled);
        }
        let build_directory = self.build_directory.clone();
        let sources = self.sources.clone();
        let deadline = Instant::now() + self.timeout;
        let timeout = self.timeout;
        let worker = tokio::task::spawn_blocking(move || {
            scan_sources(request, build_directory, sources, cancellation, deadline)
        });
        match tokio::time::timeout(timeout, worker).await {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => Err(RootfsCompositionAdapterError::Io(format!(
                "scan worker failed: {error}"
            ))),
            Err(_) => Err(RootfsCompositionAdapterError::Timeout(timeout.as_secs())),
        }
    }
}
