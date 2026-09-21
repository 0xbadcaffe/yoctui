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
