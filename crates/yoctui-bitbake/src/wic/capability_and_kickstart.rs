async fn spawn_async_command(command: &mut Command) -> std::io::Result<Child> {
    for attempt in 1..=WIC_SPAWN_ATTEMPTS {
        match command.spawn() {
            Ok(child) => return Ok(child),
            Err(error) if attempt < WIC_SPAWN_ATTEMPTS && is_transient_spawn_error(&error) => {
                tokio::time::sleep(WIC_SPAWN_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the bounded Wic spawn loop always returns")
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WicAdapterError {
    #[error("unsafe Wic executable: {0}")]
    UnsafeExecutable(PathBuf),
    #[error("unsafe Wic kickstart: {0}")]
    UnsafeKickstart(PathBuf),
    #[error("unsafe Wic output directory: {0}")]
    UnsafeOutputDirectory(PathBuf),
    #[error("Wic capability command failed: {0}")]
    Capability(String),
    #[error("invalid Wic request: {0}")]
    InvalidRequest(String),
    #[error("Wic preview does not match the independently validated command")]
    PreviewMismatch,
    #[error("a Wic process or unconsumed terminal event is already active")]
    Busy,
    #[error("could not start Wic: {0}")]
    Spawn(String),
    #[error("Wic runner is not active")]
    NotRunning,
    #[error("Wic process control failed: {0}")]
    ProcessControl(String),
    #[error("Wic output scan failed: {0}")]
    OutputScan(String),
    #[error("Wic device discovery tool is unavailable: {0}")]
    MissingDeviceTool(PathBuf),
    #[error("Wic device discovery failed: {0}")]
    DeviceDiscovery(String),
    #[error("unsafe Wic image identity: {0}")]
    UnsafeImage(PathBuf),
    #[error("unsafe Wic device identity: {0}")]
    UnsafeDevice(PathBuf),
    #[error("the Wic device identity changed since discovery")]
    StaleDevice,
}

#[derive(Debug, Clone)]
pub struct WicCapabilityInspector {
    executable: PathBuf,
    configured_kickstarts: Vec<PathBuf>,
    canned_roots: Vec<PathBuf>,
}

impl Default for WicCapabilityInspector {
    fn default() -> Self {
        Self {
            executable: "wic".into(),
            configured_kickstarts: Vec::new(),
            canned_roots: Vec::new(),
        }
    }
}

impl WicCapabilityInspector {
    pub fn with_executable(executable: PathBuf) -> Self {
        Self {
            executable,
            ..Self::default()
        }
    }

    pub fn with_sources(
        mut self,
        configured_kickstarts: Vec<PathBuf>,
        canned_roots: Vec<PathBuf>,
    ) -> Self {
        self.configured_kickstarts = configured_kickstarts;
        self.canned_roots = canned_roots;
        self
    }

    pub async fn inspect(&self, image_targets: Vec<String>) -> WicCapability {
        let executable = match resolve_executable(&self.executable) {
            Ok(Some(executable)) => executable,
            Ok(None) => return WicCapability::MissingTool,
            Err(message) => return WicCapability::Failed { message },
        };
        let listed = match list_canned(&executable).await {
            Ok(listed) => listed,
            Err(error) => {
                return WicCapability::Failed {
                    message: error.to_string(),
                };
            }
        };
        let mut kickstarts = Vec::new();
        for path in &self.configured_kickstarts {
            match read_kickstart(path, None) {
                Ok(kickstart) => kickstarts.push(kickstart),
                Err(error) => {
                    return WicCapability::Failed {
                        message: error.to_string(),
                    };
                }
            }
        }
        for name in listed.into_iter().take(MAX_WIC_KICKSTARTS) {
            let path = self.canned_roots.iter().find_map(|root| {
                [
                    root.join(format!("{name}.wks")),
                    root.join(format!("{name}.wks.in")),
                ]
                .into_iter()
                .find(|path| path.exists())
            });
            match path {
                Some(path) => match read_kickstart(&path, Some(name)) {
                    Ok(kickstart) => kickstarts.push(kickstart),
                    Err(error) => {
                        return WicCapability::Failed {
                            message: error.to_string(),
                        };
                    }
                },
                None => kickstarts.push(WicKickstart {
                    identity: WicKickstartIdentity { name, path: None },
                    source: String::new(),
                    partitions: Vec::new(),
                    limitations: vec!["canned kickstart source is unavailable".into()],
                }),
            }
        }
        normalize_wic_capability(WicCapability::Available {
            executable,
            kickstarts,
            image_targets,
        })
    }
}

async fn list_canned(executable: &Path) -> Result<Vec<String>, WicAdapterError> {
    let mut command = Command::new(executable);
    command
        .args(["list", "images"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = spawn_async_command(&mut command)
        .await
        .map_err(|error| WicAdapterError::Capability(error.to_string()))?;
    let stdout = child.stdout.take().ok_or_else(|| {
        WicAdapterError::Capability("wic list images stdout is unavailable".into())
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        WicAdapterError::Capability("wic list images stderr is unavailable".into())
    })?;
    let read = async move {
        let mut stdout_bytes = Vec::new();
        let mut stderr_bytes = Vec::new();
        let mut bounded_stdout = stdout.take(MAX_WIC_LIST_BYTES + 1);
        let mut bounded_stderr = stderr.take(MAX_WIC_LIST_BYTES + 1);
        let stdout_read = bounded_stdout.read_to_end(&mut stdout_bytes);
        let stderr_read = bounded_stderr.read_to_end(&mut stderr_bytes);
        let (stdout_result, stderr_result, status) =
            tokio::join!(stdout_read, stderr_read, child.wait());
        stdout_result.map_err(|error| WicAdapterError::Capability(error.to_string()))?;
        stderr_result.map_err(|error| WicAdapterError::Capability(error.to_string()))?;
        let status = status.map_err(|error| WicAdapterError::Capability(error.to_string()))?;
        if stdout_bytes.len() as u64 > MAX_WIC_LIST_BYTES
            || stderr_bytes.len() as u64 > MAX_WIC_LIST_BYTES
        {
            return Err(WicAdapterError::Capability(
                "wic list images output exceeded its safety bound".into(),
            ));
        }
        if !status.success() {
            return Err(WicAdapterError::Capability(
                String::from_utf8_lossy(&stderr_bytes).trim().to_owned(),
            ));
        }
        let output = String::from_utf8(stdout_bytes)
            .map_err(|error| WicAdapterError::Capability(error.to_string()))?;
        let mut names = Vec::new();
        for line in output.lines().filter(|line| !line.trim().is_empty()) {
            let Some(name) = line.split_ascii_whitespace().next() else {
                continue;
            };
            if name.len() <= 256
                && name.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
                })
            {
                names.push(name.to_owned());
            } else {
                return Err(WicAdapterError::Capability(
                    "wic list images returned a malformed name".into(),
                ));
            }
        }
        names.sort();
        names.dedup();
        Ok(names)
    };
    tokio::time::timeout(WIC_INSPECTION_TIMEOUT, read)
        .await
        .map_err(|_| WicAdapterError::Capability("wic list images timed out".into()))?
}

fn read_kickstart(
    path: &Path,
    canned_name: Option<String>,
) -> Result<WicKickstart, WicAdapterError> {
    let canonical =
        regular_canonical(path).map_err(|_| WicAdapterError::UnsafeKickstart(path.into()))?;
    let bytes = fs::read(&canonical).map_err(|_| WicAdapterError::UnsafeKickstart(path.into()))?;
    if bytes.len() > MAX_WIC_SOURCE_BYTES {
        return Err(WicAdapterError::UnsafeKickstart(path.into()));
    }
    let source =
        String::from_utf8(bytes).map_err(|_| WicAdapterError::UnsafeKickstart(path.into()))?;
    let name = canned_name.unwrap_or_else(|| {
        canonical
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .trim_end_matches(".in")
            .trim_end_matches(".wks")
            .to_owned()
    });
    let (partitions, limitations) = parse_kickstart(&source);
    WicKickstart {
        identity: WicKickstartIdentity {
            name,
            path: Some(canonical),
        },
        source,
        partitions,
        limitations,
    }
    .normalize()
    .map_err(|_| WicAdapterError::UnsafeKickstart(path.into()))
}

fn parse_kickstart(source: &str) -> (Vec<WicPartitionSummary>, Vec<String>) {
    let mut partitions = Vec::new();
    let mut limitations = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut tokens = line.split_ascii_whitespace();
        let Some(command) = tokens.next() else {
            continue;
        };
        if !matches!(command, "part" | "partition") {
            if command != "bootloader" {
                limitations.push(format!("unsupported kickstart command: {command}"));
            }
            continue;
        }
        let mount_point = tokens
            .next()
            .filter(|value| !value.starts_with("--"))
            .map(str::to_owned);
        let mut partition = WicPartitionSummary {
            mount_point,
            filesystem: None,
            source_plugin: None,
            size_mib: None,
            alignment_kib: None,
        };
        for token in line.split_ascii_whitespace().skip(1) {
            if let Some(value) = token.strip_prefix("--fstype=") {
                partition.filesystem = Some(value.into());
            } else if let Some(value) = token.strip_prefix("--source=") {
                partition.source_plugin = Some(value.into());
            } else if let Some(value) = token.strip_prefix("--size=") {
                partition.size_mib = value.parse().ok();
                if partition.size_mib.is_none() {
                    limitations.push("dynamic or invalid partition size".into());
                }
            } else if let Some(value) = token.strip_prefix("--align=") {
                partition.alignment_kib = value.parse().ok();
                if partition.alignment_kib.is_none() {
                    limitations.push("dynamic or invalid partition alignment".into());
                }
            } else if token.contains("${") {
                limitations.push("variable-derived partition option".into());
            }
        }
        partitions.push(partition);
    }
    (partitions, limitations)
}
