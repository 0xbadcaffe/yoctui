use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use thiserror::Error;
use tokio::{io::AsyncReadExt, process::Command};
use yoctui_bitbake::{
    CapabilityCacheError, CapabilityFingerprintMaterial, CapabilityProbeContext,
    CapabilityProbeObservation, CapabilityProbeRunner, CapabilityResolver, CapabilitySnapshotCache,
};
use yoctui_model::{
    AuthoritativeValue, CapabilityCacheKey, CapabilityCatalog, CapabilityCatalogError,
    CapabilityId, CapabilityImplementation, CapabilityToolId, DaemonCompatibilitySnapshot,
    DistroIdentity, IdentityAuthority, LayerSeriesIdentity, ProtocolIdentity, ReleaseIdentity,
    SourceRootIdentity, SourceRootKind, ToolIdentity, YoctoEnvironmentIdentity,
};

const STARTUP_QUERY_TIMEOUT: Duration = Duration::from_secs(30);
const STARTUP_QUERY_OUTPUT_LIMIT: usize = 64 * 1024;
const STARTUP_FINGERPRINT_LIMIT: usize = 4 * 1024 * 1024;
const STARTUP_ENVIRONMENT_LIMIT: usize = 1_024;
const STARTUP_ENVIRONMENT_VALUE_LIMIT: usize = 4_096;
const PROBE_CONCURRENCY: usize = 8;

#[derive(Debug, Clone)]
pub struct DaemonCompatibilityRuntime {
    pub key: CapabilityCacheKey,
    pub context: CapabilityProbeContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonCompatibilityProbeTicket {
    pub key: CapabilityCacheKey,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonCompatibilitySelection {
    Cached(DaemonCompatibilitySnapshot),
    Probe(DaemonCompatibilityProbeTicket),
}

/// Sole daemon owner of environment-correlated capability probing and cache
/// state. Clients receive its resolved snapshots through the daemon journal;
/// they never run this coordinator or infer release support themselves.
#[derive(Debug, Clone)]
pub struct DaemonCompatibilityCoordinator {
    cache: CapabilitySnapshotCache,
    catalog: CapabilityCatalog,
    resolver: CapabilityResolver,
    runner: CapabilityProbeRunner,
    active_key: Option<CapabilityCacheKey>,
    implementations: BTreeMap<CapabilityId, CapabilityImplementation>,
}

impl Default for DaemonCompatibilityCoordinator {
    fn default() -> Self {
        Self {
            cache: CapabilitySnapshotCache::default(),
            catalog: CapabilityCatalog::builtin(),
            resolver: CapabilityResolver::default(),
            runner: CapabilityProbeRunner::default(),
            active_key: None,
            implementations: BTreeMap::new(),
        }
    }
}

impl DaemonCompatibilityCoordinator {
    pub async fn startup_from_environment(
        &mut self,
        environment: &BTreeMap<String, String>,
    ) -> Result<Option<DaemonCompatibilitySnapshot>, DaemonCompatibilityError> {
        let Some(runtime) = DaemonCompatibilityRuntime::detect(environment).await? else {
            return Ok(None);
        };
        match self.select_environment(runtime.key)? {
            DaemonCompatibilitySelection::Cached(snapshot) => Ok(Some(snapshot)),
            DaemonCompatibilitySelection::Probe(ticket) => {
                self.probe(ticket, &runtime.context).await.map(Some)
            }
        }
    }

    pub fn select_environment(
        &mut self,
        key: CapabilityCacheKey,
    ) -> Result<DaemonCompatibilitySelection, DaemonCompatibilityError> {
        self.catalog.validate()?;
        let selection = self.cache.select(key.clone())?;
        if self.active_key.as_ref() != Some(&key) {
            self.active_key = Some(key.clone());
            self.implementations.clear();
        }
        if let Some(snapshot) = selection.snapshot {
            let compatibility = DaemonCompatibilitySnapshot {
                snapshot,
                implementations: self.implementations.clone(),
            }
            .normalize()?;
            return Ok(DaemonCompatibilitySelection::Cached(compatibility));
        }
        Ok(DaemonCompatibilitySelection::Probe(
            DaemonCompatibilityProbeTicket {
                key,
                generation: selection.generation,
            },
        ))
    }

    pub async fn probe(
        &mut self,
        ticket: DaemonCompatibilityProbeTicket,
        context: &CapabilityProbeContext,
    ) -> Result<DaemonCompatibilitySnapshot, DaemonCompatibilityError> {
        if !context.matches_environment(&ticket.key.environment) {
            return Err(DaemonCompatibilityError::EnvironmentMismatch);
        }
        let mut unique_probes = Vec::<yoctui_model::CapabilityProbeSpec>::new();
        for entry in &self.catalog.entries {
            for probe in &entry.probes {
                if !unique_probes.contains(probe) {
                    unique_probes.push(probe.clone());
                }
            }
        }
        let semaphore = Arc::new(tokio::sync::Semaphore::new(PROBE_CONCURRENCY));
        let mut tasks = tokio::task::JoinSet::new();
        for (index, probe) in unique_probes.iter().cloned().enumerate() {
            let semaphore = Arc::clone(&semaphore);
            let runner = self.runner.clone();
            let context = context.clone();
            tasks.spawn(async move {
                let _permit = semaphore
                    .acquire_owned()
                    .await
                    .map_err(|error| DaemonCompatibilityError::StartupProbe(error.to_string()))?;
                Ok::<_, DaemonCompatibilityError>((index, runner.probe(&context, &probe).await))
            });
        }
        let mut completed = vec![None; unique_probes.len()];
        while let Some(result) = tasks.join_next().await {
            let (index, observation) = result
                .map_err(|error| DaemonCompatibilityError::StartupProbe(error.to_string()))??;
            completed[index] = Some(observation);
        }
        let mut observations = BTreeMap::<CapabilityId, Vec<CapabilityProbeObservation>>::new();
        for entry in &self.catalog.entries {
            observations.insert(
                entry.id,
                entry
                    .probes
                    .iter()
                    .map(|probe| {
                        let index = unique_probes
                            .iter()
                            .position(|candidate| candidate == probe)
                            .expect("catalog probe was indexed before execution");
                        completed[index]
                            .clone()
                            .expect("every indexed probe task must complete")
                    })
                    .collect(),
            );
        }
        let resolved = self.resolver.resolve_snapshot(
            ticket.generation,
            ticket.key.environment.clone(),
            &self.catalog,
            &observations,
        )?;
        self.accept(ticket, resolved.snapshot, resolved.implementations)
    }

    pub fn accept(
        &mut self,
        ticket: DaemonCompatibilityProbeTicket,
        snapshot: yoctui_model::CapabilitySnapshot,
        implementations: BTreeMap<CapabilityId, CapabilityImplementation>,
    ) -> Result<DaemonCompatibilitySnapshot, DaemonCompatibilityError> {
        if self.active_key.as_ref() != Some(&ticket.key)
            || self.cache.generation() != ticket.generation
        {
            return Err(DaemonCompatibilityError::StaleProbe);
        }
        let compatibility = DaemonCompatibilitySnapshot {
            snapshot,
            implementations,
        }
        .normalize()?;
        self.cache.store(
            &ticket.key,
            ticket.generation,
            compatibility.snapshot.clone(),
        )?;
        self.implementations = compatibility.implementations.clone();
        Ok(compatibility)
    }

    pub fn invalidate(&mut self) -> Result<u64, DaemonCompatibilityError> {
        self.active_key = None;
        self.implementations.clear();
        Ok(self.cache.invalidate()?)
    }
}

impl DaemonCompatibilityRuntime {
    pub async fn detect(
        process_environment: &BTreeMap<String, String>,
    ) -> Result<Option<Self>, DaemonCompatibilityError> {
        let Some(configured_build) = process_environment.get("BUILDDIR") else {
            return Ok(None);
        };
        let build_directory = canonical_initialized_build(configured_build)?;
        let catalog = CapabilityCatalog::builtin();
        catalog.validate()?;
        let tool_ids = catalog
            .entries
            .iter()
            .flat_map(|entry| entry.required_tools.iter().copied())
            .collect::<BTreeSet<_>>();
        let path = process_environment.get("PATH").ok_or(
            DaemonCompatibilityError::InvalidStartupEnvironment(
                "initialized environment has no PATH".into(),
            ),
        )?;
        let mut tools = BTreeMap::new();
        for tool in tool_ids {
            if let Some(executable) = discover_executable(path, tool.executable_name()) {
                tools.insert(tool, executable);
            }
        }

        let bitbake_version = if let Some(bitbake) = tools.get(&CapabilityToolId::BitBake) {
            run_read_only(
                bitbake,
                &["--version"],
                &build_directory,
                process_environment,
            )
            .await
            .ok()
            .and_then(|output| parse_bitbake_version(&output))
        } else {
            None
        };

        let mut datastore = BTreeMap::new();
        if let Some(getvar) = tools.get(&CapabilityToolId::BitBakeGetVar) {
            for variable in [
                "MACHINE",
                "DISTRO",
                "DISTRO_VERSION",
                "DISTRO_CODENAME",
                "OE_VERSION",
                "COREBASE",
                "LAYERSERIES_CORENAMES",
                "BBLAYERS",
                "BB_HASHSERVE",
                "PRSERV_HOST",
            ] {
                if let Ok(value) = run_read_only(
                    getvar,
                    &["--value", variable],
                    &build_directory,
                    process_environment,
                )
                .await
                {
                    datastore.insert(variable.to_owned(), value.trim().to_owned());
                }
            }
        }

        let available_tools = tools
            .iter()
            .map(|(id, executable)| ToolIdentity {
                id: id.executable_name().into(),
                executable: executable.clone(),
                version: (*id == CapabilityToolId::BitBake)
                    .then(|| bitbake_version.clone())
                    .flatten(),
            })
            .collect::<Vec<_>>();
        let source_roots = datastore
            .get("BBLAYERS")
            .into_iter()
            .flat_map(|value| value.split_whitespace())
            .filter_map(|value| fs::canonicalize(value).ok())
            .filter(|path| path.is_dir())
            .map(|path| SourceRootIdentity {
                kind: SourceRootKind::Layer,
                path,
            })
            .collect::<Vec<_>>();
        let distro = datastore
            .get("DISTRO")
            .filter(|value| !value.is_empty())
            .map(|name| DistroIdentity {
                name: name.clone(),
                version: datastore
                    .get("DISTRO_VERSION")
                    .filter(|value| !value.is_empty())
                    .cloned(),
            });
        let poky = (datastore.get("DISTRO").map(String::as_str) == Some("poky"))
            .then(|| ReleaseIdentity {
                name: datastore
                    .get("DISTRO_CODENAME")
                    .filter(|value| !value.is_empty())
                    .cloned(),
                version: datastore
                    .get("DISTRO_VERSION")
                    .filter(|value| !value.is_empty())
                    .cloned(),
            })
            .filter(|release| release.name.is_some() || release.version.is_some());
        let oe_core = datastore
            .get("LAYERSERIES_CORENAMES")
            .or_else(|| datastore.get("OE_VERSION"))
            .map(|_| ReleaseIdentity {
                name: datastore
                    .get("LAYERSERIES_CORENAMES")
                    .and_then(|value| value.split_whitespace().next())
                    .map(str::to_owned),
                version: datastore
                    .get("OE_VERSION")
                    .filter(|value| !value.is_empty())
                    .cloned(),
            })
            .filter(|release| release.name.is_some() || release.version.is_some());
        let layer_series = datastore
            .get("COREBASE")
            .filter(|value| !value.is_empty())
            .and_then(|value| fs::canonicalize(Path::new(value).join("meta")).ok())
            .filter(|root| root.is_dir())
            .zip(
                datastore
                    .get("LAYERSERIES_CORENAMES")
                    .map(|value| {
                        value
                            .split_whitespace()
                            .map(str::to_owned)
                            .collect::<Vec<_>>()
                    })
                    .filter(|series| !series.is_empty()),
            )
            .map(|(root, compatible_series)| {
                vec![LayerSeriesIdentity {
                    layer: "core".into(),
                    root,
                    compatible_series,
                }]
            });
        let identity = YoctoEnvironmentIdentity {
            build_directory: AuthoritativeValue::detected(
                build_directory.clone(),
                IdentityAuthority::InitializedEnvironment,
            ),
            source_roots: if source_roots.is_empty() {
                AuthoritativeValue::Unknown
            } else {
                AuthoritativeValue::detected(
                    source_roots,
                    IdentityAuthority::ConfiguredLayerMetadata,
                )
            },
            bitbake_version: bitbake_version.map_or(AuthoritativeValue::Unknown, |version| {
                AuthoritativeValue::detected(version, IdentityAuthority::BitBakeVersionProbe)
            }),
            oe_core: oe_core.map_or(AuthoritativeValue::Unknown, |release| {
                AuthoritativeValue::detected(release, IdentityAuthority::BitBakeDatastore)
            }),
            poky: poky.map_or(AuthoritativeValue::Unknown, |release| {
                AuthoritativeValue::detected(release, IdentityAuthority::BitBakeDatastore)
            }),
            distro: distro.map_or(AuthoritativeValue::Unknown, |distro| {
                AuthoritativeValue::detected(distro, IdentityAuthority::BitBakeDatastore)
            }),
            machine: datastore
                .get("MACHINE")
                .filter(|value| !value.is_empty())
                .cloned()
                .map_or(AuthoritativeValue::Unknown, |machine| {
                    AuthoritativeValue::detected(machine, IdentityAuthority::BitBakeDatastore)
                }),
            layer_series: layer_series.map_or(AuthoritativeValue::Unknown, |layers| {
                AuthoritativeValue::detected(layers, IdentityAuthority::ConfiguredLayerMetadata)
            }),
            available_tools: AuthoritativeValue::detected(
                available_tools,
                IdentityAuthority::ExecutableProbe,
            ),
            protocol: AuthoritativeValue::detected(
                ProtocolIdentity {
                    name: "yoctui-daemon".into(),
                    version: format!(
                        "{}.{}",
                        yoctui_protocol::daemon::ProtocolVersion::CURRENT.major,
                        yoctui_protocol::daemon::ProtocolVersion::CURRENT.minor
                    ),
                },
                IdentityAuthority::ProtocolNegotiation,
            ),
            ..YoctoEnvironmentIdentity::default()
        }
        .normalize()?;

        let bounded_environment = bounded_process_environment(process_environment);
        let metadata_variables = datastore.keys().cloned().collect();
        let artifacts = [
            ("pkgdata", build_directory.join("tmp/pkgdata")),
            ("wic", build_directory.join("tmp/deploy/images")),
        ]
        .into_iter()
        .filter(|(_, path)| path.is_dir())
        .map(|(name, _)| name.to_owned())
        .collect();
        let context = CapabilityProbeContext::new(
            identity.clone(),
            build_directory.clone(),
            tools,
            bounded_environment,
            None,
            Some(metadata_variables),
            None,
            Some(BTreeSet::from(["state_snapshots".into()])),
            Some(artifacts),
            None,
        )?;
        let layer_configuration = read_bounded(&build_directory.join("conf/bblayers.conf"))?;
        let build_configuration = read_bounded(&build_directory.join("conf/local.conf"))?;
        let initialized_environment = fingerprint_environment(process_environment)?;
        let workspace = build_directory.to_string_lossy().into_owned();
        let key = CapabilityFingerprintMaterial {
            workspace_identity: &workspace,
            initialized_environment: &initialized_environment,
            layer_configuration: &layer_configuration,
            build_configuration: &build_configuration,
            daemon_workspace_identity: &workspace,
        }
        .key(identity)?;
        Ok(Some(Self { key, context }))
    }
}

fn canonical_initialized_build(value: &str) -> Result<PathBuf, DaemonCompatibilityError> {
    let configured = Path::new(value);
    let build = fs::canonicalize(configured).map_err(|error| {
        DaemonCompatibilityError::InvalidStartupEnvironment(format!(
            "BUILDDIR {} cannot be resolved: {error}",
            configured.display()
        ))
    })?;
    if !build.is_dir()
        || build == Path::new("/")
        || !build.join("conf/local.conf").is_file()
        || !build.join("conf/bblayers.conf").is_file()
    {
        return Err(DaemonCompatibilityError::InvalidStartupEnvironment(
            format!(
                "BUILDDIR {} is not an initialized Yocto build",
                build.display()
            ),
        ));
    }
    Ok(build)
}

fn discover_executable(path: &str, name: &str) -> Option<PathBuf> {
    std::env::split_paths(path).find_map(|directory| {
        let candidate = directory.join(name);
        if !candidate.is_absolute() || fs::canonicalize(&directory).ok()? != directory {
            return None;
        }
        let metadata = fs::symlink_metadata(&candidate).ok()?;
        let executable_metadata = if metadata.file_type().is_symlink() {
            let target = fs::read_link(&candidate).ok()?;
            if target.is_absolute()
                || target
                    .components()
                    .any(|component| !matches!(component, std::path::Component::Normal(_)))
            {
                return None;
            }
            let canonical_target = fs::canonicalize(directory.join(target)).ok()?;
            if canonical_target.parent()? != directory {
                return None;
            }
            fs::metadata(canonical_target).ok()?
        } else {
            metadata
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            (executable_metadata.is_file() && executable_metadata.permissions().mode() & 0o111 != 0)
                .then_some(candidate)
        }
        #[cfg(not(unix))]
        {
            executable_metadata.is_file().then_some(candidate)
        }
    })
}

async fn run_read_only(
    executable: &Path,
    arguments: &[&str],
    build_directory: &Path,
    environment: &BTreeMap<String, String>,
) -> Result<String, DaemonCompatibilityError> {
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .current_dir(build_directory)
        .env_clear()
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().map_err(|error| {
        DaemonCompatibilityError::StartupProbe(format!(
            "could not start {}: {error}",
            executable.display()
        ))
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| DaemonCompatibilityError::StartupProbe("stdout unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| DaemonCompatibilityError::StartupProbe("stderr unavailable".into()))?;
    let stdout_task = tokio::spawn(read_bounded_stream(stdout));
    let stderr_task = tokio::spawn(read_bounded_stream(stderr));
    let status = match tokio::time::timeout(STARTUP_QUERY_TIMEOUT, child.wait()).await {
        Ok(status) => status.map_err(|error| {
            DaemonCompatibilityError::StartupProbe(format!(
                "could not wait for {}: {error}",
                executable.display()
            ))
        })?,
        Err(_) => {
            #[cfg(unix)]
            if let Some(pid) = child.id() {
                // The command was placed in its own process group above.
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
            }
            let _ = child.kill().await;
            return Err(DaemonCompatibilityError::StartupProbe(format!(
                "read-only query timed out for {}",
                executable.display()
            )));
        }
    };
    let (stdout, stdout_truncated) = stdout_task
        .await
        .map_err(|error| DaemonCompatibilityError::StartupProbe(error.to_string()))??;
    let (stderr, stderr_truncated) = stderr_task
        .await
        .map_err(|error| DaemonCompatibilityError::StartupProbe(error.to_string()))??;
    if stdout_truncated || stderr_truncated {
        return Err(DaemonCompatibilityError::StartupProbe(format!(
            "read-only query output exceeded {} bytes per stream",
            STARTUP_QUERY_OUTPUT_LIMIT
        )));
    }
    if !status.success() {
        return Err(DaemonCompatibilityError::StartupProbe(format!(
            "read-only query failed for {}: {}",
            executable.display(),
            String::from_utf8_lossy(&stderr).trim()
        )));
    }
    let mut output = String::from_utf8_lossy(&stdout).into_owned();
    if output.trim().is_empty() {
        output = String::from_utf8_lossy(&stderr).into_owned();
    }
    Ok(output)
}

async fn read_bounded_stream<R>(mut stream: R) -> Result<(Vec<u8>, bool), DaemonCompatibilityError>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut retained = Vec::new();
    let mut buffer = [0_u8; 8 * 1024];
    let mut truncated = false;
    loop {
        let count = stream.read(&mut buffer).await.map_err(|error| {
            DaemonCompatibilityError::StartupProbe(format!("could not read query output: {error}"))
        })?;
        if count == 0 {
            break;
        }
        let remaining = STARTUP_QUERY_OUTPUT_LIMIT.saturating_sub(retained.len());
        retained.extend_from_slice(&buffer[..count.min(remaining)]);
        truncated |= count > remaining;
    }
    Ok((retained, truncated))
}

fn parse_bitbake_version(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let line = line.trim();
        line.contains("BitBake")
            .then(|| line.split_whitespace().next_back())
            .flatten()
            .filter(|value| {
                value
                    .chars()
                    .next()
                    .is_some_and(|value| value.is_ascii_digit())
                    && value.chars().all(|value| {
                        value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '+')
                    })
            })
            .map(str::to_owned)
    })
}

fn bounded_process_environment(environment: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let valid = |key: &str, value: &str| {
        !key.is_empty()
            && !value.is_empty()
            && key.len() <= STARTUP_ENVIRONMENT_VALUE_LIMIT
            && value.len() <= STARTUP_ENVIRONMENT_VALUE_LIMIT
            && !key.contains('\0')
            && !value.contains('\0')
    };
    let mut bounded = BTreeMap::new();
    for key in ["BUILDDIR", "PATH", "BBPATH", "PYTHONPATH", "HOME"] {
        if let Some(value) = environment.get(key)
            && valid(key, value)
        {
            bounded.insert(key.to_owned(), value.clone());
        }
    }
    for (key, value) in environment {
        if bounded.len() >= STARTUP_ENVIRONMENT_LIMIT {
            break;
        }
        if valid(key, value) {
            bounded.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
    bounded
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, DaemonCompatibilityError> {
    let metadata = fs::metadata(path).map_err(|error| {
        DaemonCompatibilityError::InvalidStartupEnvironment(format!(
            "could not inspect {}: {error}",
            path.display()
        ))
    })?;
    if metadata.len() > STARTUP_FINGERPRINT_LIMIT as u64 {
        return Err(DaemonCompatibilityError::InvalidStartupEnvironment(
            format!(
                "{} exceeds the startup fingerprint safety bound",
                path.display()
            ),
        ));
    }
    fs::read(path).map_err(|error| {
        DaemonCompatibilityError::InvalidStartupEnvironment(format!(
            "could not read {}: {error}",
            path.display()
        ))
    })
}

fn fingerprint_environment(
    environment: &BTreeMap<String, String>,
) -> Result<Vec<u8>, DaemonCompatibilityError> {
    let mut encoded = Vec::new();
    for (key, value) in environment {
        if encoded
            .len()
            .saturating_add(key.len())
            .saturating_add(value.len())
            .saturating_add(2)
            > STARTUP_FINGERPRINT_LIMIT
        {
            return Err(DaemonCompatibilityError::InvalidStartupEnvironment(
                "initialized environment exceeds the startup fingerprint safety bound".into(),
            ));
        }
        encoded.extend_from_slice(key.as_bytes());
        encoded.push(b'=');
        encoded.extend_from_slice(value.as_bytes());
        encoded.push(0);
    }
    Ok(encoded)
}

#[derive(Debug, Error)]
pub enum DaemonCompatibilityError {
    #[error(transparent)]
    Cache(#[from] CapabilityCacheError),
    #[error(transparent)]
    Catalog(#[from] CapabilityCatalogError),
    #[error(transparent)]
    Model(#[from] yoctui_model::CapabilityModelError),
    #[error(transparent)]
    State(#[from] yoctui_model::DaemonStateError),
    #[error(transparent)]
    ProbeContext(#[from] yoctui_bitbake::CapabilityProbeContextError),
    #[error(transparent)]
    Identity(#[from] yoctui_model::EnvironmentIdentityError),
    #[error("invalid initialized daemon environment: {0}")]
    InvalidStartupEnvironment(String),
    #[error("daemon startup capability query failed: {0}")]
    StartupProbe(String),
    #[error("capability probe context belongs to another environment")]
    EnvironmentMismatch,
    #[error("capability probe result is stale for the selected daemon environment")]
    StaleProbe,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::{BTreeMap, BTreeSet},
        fs,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    use yoctui_bitbake::CapabilityFingerprintMaterial;
    use yoctui_model::{AuthoritativeValue, IdentityAuthority, YoctoEnvironmentIdentity};

    static NEXT_RUNTIME: AtomicU64 = AtomicU64::new(1);

    fn environment(build: PathBuf, version: &str) -> YoctoEnvironmentIdentity {
        YoctoEnvironmentIdentity {
            build_directory: AuthoritativeValue::detected(
                build,
                IdentityAuthority::InitializedEnvironment,
            ),
            bitbake_version: AuthoritativeValue::detected(
                version.into(),
                IdentityAuthority::BitBakeVersionProbe,
            ),
            ..YoctoEnvironmentIdentity::default()
        }
    }

    fn key(environment: YoctoEnvironmentIdentity, workspace: &str) -> CapabilityCacheKey {
        CapabilityFingerprintMaterial {
            workspace_identity: workspace,
            initialized_environment: b"PATH=/work/poky/bitbake/bin",
            layer_configuration: b"BBLAYERS=/work/poky/meta",
            build_configuration: b"MACHINE=qemux86-64\nDISTRO=poky",
            daemon_workspace_identity: workspace,
        }
        .key(environment)
        .unwrap()
    }

    fn context(environment: YoctoEnvironmentIdentity) -> CapabilityProbeContext {
        CapabilityProbeContext::new(
            environment.clone(),
            environment.build_directory.value().unwrap().clone(),
            BTreeMap::new(),
            BTreeMap::new(),
            Some(BTreeSet::from([
                "do_build".into(),
                "do_populate_sdk".into(),
            ])),
            Some(BTreeSet::from(["MACHINE".into(), "DISTRO".into()])),
            Some(BTreeSet::from(["workspace_inspection".into()])),
            Some(BTreeSet::from(["state_snapshots".into()])),
            Some(BTreeSet::new()),
            Some(BTreeSet::new()),
        )
        .unwrap()
    }

    fn temporary_build(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "yoctui-daemon-compatibility-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path.canonicalize().unwrap()
    }

    struct RuntimeFixture {
        root: PathBuf,
        build: PathBuf,
        bin: PathBuf,
        environment: BTreeMap<String, String>,
    }

    impl RuntimeFixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "yoctui-daemon-compatibility-runtime-{}-{}",
                std::process::id(),
                NEXT_RUNTIME.fetch_add(1, Ordering::Relaxed)
            ));
            let build = root.join("build");
            let bin = root.join("bin");
            let layer = root.join("layers/meta");
            fs::create_dir_all(build.join("conf")).unwrap();
            fs::create_dir_all(&bin).unwrap();
            fs::create_dir_all(&layer).unwrap();
            fs::write(
                build.join("conf/local.conf"),
                "MACHINE = \"qemux86-64\"\nDISTRO = \"poky\"\n",
            )
            .unwrap();
            fs::write(
                build.join("conf/bblayers.conf"),
                format!("BBLAYERS = \"{}\"\n", layer.display()),
            )
            .unwrap();
            write_tool(
                &bin.join("bitbake"),
                "case \"$1\" in\n  --version) echo 'BitBake Build Tool Core version 2.18.0' ;;\n  --help) echo 'usage: bitbake -e -g -f -c --dry-run --status-only --server-only --kill-server' ;;\n  *) echo ok ;;\nesac",
            );
            write_tool(
                &bin.join("bitbake-getvar"),
                &format!(
                    "if [ \"$1\" = --help ]; then echo 'usage: bitbake-getvar --value --recipe'; exit 0; fi\ncase \"$2\" in\n  MACHINE) echo qemux86-64 ;;\n  DISTRO) echo poky ;;\n  DISTRO_VERSION) echo 6.0.2 ;;\n  DISTRO_CODENAME) echo wrynose ;;\n  OE_VERSION) echo 5.3 ;;\n  COREBASE) echo '{}' ;;\n  LAYERSERIES_CORENAMES) echo wrynose ;;\n  BBLAYERS) echo '{}' ;;\n  BB_HASHSERVE|PRSERV_HOST) echo '' ;;\nesac",
                    layer.parent().unwrap().display(),
                    layer.display()
                ),
            );
            let environment = BTreeMap::from([
                ("BUILDDIR".into(), build.display().to_string()),
                ("PATH".into(), bin.display().to_string()),
                ("HOME".into(), root.display().to_string()),
            ]);
            Self {
                root,
                build,
                bin,
                environment,
            }
        }
    }

    impl Drop for RuntimeFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn write_tool(path: &Path, body: &str) {
        fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[tokio::test]
    async fn daemon_compatibility_runtime_doctor_compatibility_publishes_initialized_authority() {
        let fixture = RuntimeFixture::new();
        let mut coordinator = DaemonCompatibilityCoordinator::default();
        let compatibility = coordinator
            .startup_from_environment(&fixture.environment)
            .await
            .unwrap()
            .expect("initialized environment must produce authority");
        assert_eq!(
            compatibility.snapshot.environment.build_directory.value(),
            Some(&fixture.build.canonicalize().unwrap())
        );
        assert_eq!(
            compatibility.snapshot.environment.bitbake_version.value(),
            Some(&"2.18.0".to_owned())
        );
        assert_eq!(
            compatibility.snapshot.environment.machine.value(),
            Some(&"qemux86-64".to_owned())
        );
        assert_eq!(
            compatibility
                .snapshot
                .environment
                .layer_series
                .value()
                .unwrap(),
            &[LayerSeriesIdentity {
                layer: "core".into(),
                root: fixture.root.join("layers/meta").canonicalize().unwrap(),
                compatible_series: vec!["wrynose".into()],
            }]
        );
        assert_eq!(
            compatibility
                .snapshot
                .environment
                .poky
                .value()
                .and_then(|release| release.name.as_deref()),
            Some("wrynose")
        );
        assert_eq!(
            compatibility
                .snapshot
                .environment
                .available_tools
                .value()
                .unwrap()
                .iter()
                .find(|tool| tool.id == "bitbake-getvar")
                .unwrap()
                .executable,
            fixture.bin.join("bitbake-getvar").canonicalize().unwrap()
        );
        assert!(compatibility.snapshot.allows(CapabilityId::BitBakeGetVar));
        assert_eq!(
            compatibility
                .implementations
                .get(&CapabilityId::BitBakeGetVar)
                .unwrap()
                .id,
            "bitbake_getvar.argv"
        );

        let mut state = yoctui_model::DaemonGlobalState::new(
            yoctui_model::DaemonModelInstanceId([9; 16]),
            1,
            "boot".into(),
            yoctui_model::DaemonStateLimits::default(),
        )
        .unwrap();
        yoctui_app::reduce_daemon_state(
            &mut state,
            yoctui_model::DaemonStateAction::ReplaceCompatibility(Box::new(compatibility.clone())),
        )
        .unwrap();
        let wire = yoctui_app::daemon_protocol_snapshot(&state)
            .compatibility
            .expect("journal snapshot must publish compatibility");
        wire.validate().unwrap();
        assert_eq!(wire.generation, compatibility.snapshot.generation);
        assert_eq!(
            wire.environment.bitbake_version,
            yoctui_protocol::daemon::CompatibilityDetected::Detected {
                value: "2.18.0".into(),
                authority:
                    yoctui_protocol::daemon::CompatibilityIdentityAuthority::BitBakeVersionProbe,
            }
        );
        let report = crate::doctor_compatibility_report(Some(&wire), None);
        assert_eq!(
            report.authority,
            crate::DoctorCompatibilityAuthority::Current
        );
        assert!(matches!(
            report
                .environment
                .as_ref()
                .map(|environment| &environment.bitbake_version),
            Some(yoctui_protocol::daemon::CompatibilityDetected::Detected {
                value,
                authority:
                    yoctui_protocol::daemon::CompatibilityIdentityAuthority::BitBakeVersionProbe,
            }) if value == "2.18.0"
        ));
    }

    #[tokio::test]
    async fn raw_capability_probe_daemon_publishes_and_reuses_option_authority() {
        let fixture = RuntimeFixture::new();
        let mut coordinator = DaemonCompatibilityCoordinator::default();
        let first = coordinator
            .startup_from_environment(&fixture.environment)
            .await
            .unwrap()
            .unwrap();
        assert!(first.snapshot.allows(CapabilityId::BitBakeRawCli));
        assert!(first.snapshot.allows(CapabilityId::BitBakeRawDryRun));
        assert!(!first.snapshot.allows(CapabilityId::BitBakeRawRunAll));
        assert_eq!(
            first
                .implementations
                .get(&CapabilityId::BitBakeRawDryRun)
                .unwrap()
                .id,
            "bitbake.raw.dry_run.argv"
        );
        assert!(matches!(
            first
                .snapshot
                .capability(CapabilityId::BitBakeRawRunAll)
                .unwrap()
                .state,
            yoctui_model::CapabilityState::Unavailable { .. }
        ));

        let second = coordinator
            .startup_from_environment(&fixture.environment)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn daemon_compatibility_runtime_refuses_host_path_without_initialized_build() {
        let fixture = RuntimeFixture::new();
        let environment = BTreeMap::from([("PATH".into(), fixture.bin.display().to_string())]);
        assert!(
            DaemonCompatibilityCoordinator::default()
                .startup_from_environment(&environment)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn daemon_compatibility_runtime_bounds_and_rejects_invalid_initialized_input() {
        let fixture = RuntimeFixture::new();
        fs::remove_file(fixture.build.join("conf/bblayers.conf")).unwrap();
        assert!(matches!(
            DaemonCompatibilityCoordinator::default()
                .startup_from_environment(&fixture.environment)
                .await,
            Err(DaemonCompatibilityError::InvalidStartupEnvironment(_))
        ));
    }

    #[tokio::test]
    async fn daemon_compatibility_probes_once_and_reuses_one_snapshot_for_all_clients() {
        let build = temporary_build("reuse");
        let environment = environment(build.clone(), "2.18.0");
        let key = key(environment.clone(), "poky-current");
        let mut coordinator = DaemonCompatibilityCoordinator::default();
        let DaemonCompatibilitySelection::Probe(ticket) =
            coordinator.select_environment(key.clone()).unwrap()
        else {
            panic!("first environment selection must probe");
        };
        let resolved = coordinator
            .probe(ticket, &context(environment))
            .await
            .unwrap();
        assert_eq!(
            resolved.snapshot.capabilities.len(),
            CapabilityId::ALL.len()
        );

        let DaemonCompatibilitySelection::Cached(first_client) =
            coordinator.select_environment(key.clone()).unwrap()
        else {
            panic!("exact reconnect must reuse the daemon snapshot");
        };
        let DaemonCompatibilitySelection::Cached(second_client) =
            coordinator.select_environment(key).unwrap()
        else {
            panic!("second client must see the same daemon snapshot");
        };
        assert_eq!(first_client, second_client);
        assert_eq!(first_client, resolved);
        fs::remove_dir_all(build).unwrap();
    }

    #[tokio::test]
    async fn raw_capability_probe_environment_change_rejects_stale_result() {
        let first_build = temporary_build("first");
        let second_build = temporary_build("second");
        let first_environment = environment(first_build.clone(), "1.52.0");
        let second_environment = environment(second_build.clone(), "2.18.0");
        let mut coordinator = DaemonCompatibilityCoordinator::default();
        let DaemonCompatibilitySelection::Probe(stale) = coordinator
            .select_environment(key(first_environment, "poky-old"))
            .unwrap()
        else {
            panic!("first selection must probe");
        };
        let DaemonCompatibilitySelection::Probe(current) = coordinator
            .select_environment(key(second_environment.clone(), "poky-new"))
            .unwrap()
        else {
            panic!("changed environment must probe");
        };

        let current_snapshot = coordinator
            .probe(current, &context(second_environment))
            .await
            .unwrap();
        assert!(matches!(
            coordinator.accept(
                stale.clone(),
                current_snapshot.snapshot.clone(),
                current_snapshot.implementations.clone()
            ),
            Err(DaemonCompatibilityError::StaleProbe)
        ));
        assert!(matches!(
            coordinator.select_environment(stale.key).unwrap(),
            DaemonCompatibilitySelection::Probe(_)
        ));
        assert_eq!(coordinator.invalidate().unwrap(), 4);
        fs::remove_dir_all(first_build).unwrap();
        fs::remove_dir_all(second_build).unwrap();
    }
}
