use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Component, Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
};
use yoctui_model::{
    AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
    CapabilityProbeSpec, CapabilityToolId, ToolIdentity, YoctoEnvironmentIdentity,
};

const DEFAULT_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_PROBE_OUTPUT_LIMIT: usize = 64 * 1024;
const MAX_PROBE_OUTPUT_LIMIT: usize = 1024 * 1024;
const MAX_PROBE_CONTEXT_VALUES: usize = 1_024;
const MAX_PROBE_ENVIRONMENT_VALUES: usize = 1_024;
const MAX_PROBE_TEXT_BYTES: usize = 4_096;
const PROBE_SPAWN_ATTEMPTS: usize = 4;
const PROBE_SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);

#[cfg(unix)]
fn is_transient_probe_spawn_error(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ETXTBSY)
}

#[cfg(not(unix))]
fn is_transient_probe_spawn_error(_error: &io::Error) -> bool {
    false
}

async fn spawn_probe_process(command: &mut Command) -> io::Result<Child> {
    for attempt in 1..=PROBE_SPAWN_ATTEMPTS {
        match command.spawn() {
            Ok(child) => return Ok(child),
            Err(error)
                if attempt < PROBE_SPAWN_ATTEMPTS && is_transient_probe_spawn_error(&error) =>
            {
                tokio::time::sleep(PROBE_SPAWN_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the bounded compatibility probe spawn loop always returns")
}

#[derive(Debug, Clone)]
pub struct CapabilityProbeContext {
    environment: YoctoEnvironmentIdentity,
    build_directory: PathBuf,
    tools: BTreeMap<CapabilityToolId, PathBuf>,
    process_environment: BTreeMap<String, String>,
    metadata_tasks: Option<BTreeSet<String>>,
    metadata_variables: Option<BTreeSet<String>>,
    backend_capabilities: Option<BTreeSet<String>>,
    protocol_capabilities: Option<BTreeSet<String>>,
    artifacts: Option<BTreeSet<String>>,
    configurations: Option<BTreeSet<String>>,
}

impl CapabilityProbeContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        environment: YoctoEnvironmentIdentity,
        build_directory: PathBuf,
        tools: BTreeMap<CapabilityToolId, PathBuf>,
        process_environment: BTreeMap<String, String>,
        metadata_tasks: Option<BTreeSet<String>>,
        metadata_variables: Option<BTreeSet<String>>,
        backend_capabilities: Option<BTreeSet<String>>,
        protocol_capabilities: Option<BTreeSet<String>>,
        artifacts: Option<BTreeSet<String>>,
        configurations: Option<BTreeSet<String>>,
    ) -> Result<Self, CapabilityProbeContextError> {
        let environment = environment.normalize()?;
        let canonical_build = canonical_directory(&build_directory).ok_or_else(|| {
            CapabilityProbeContextError::UnsafeBuildDirectory(build_directory.clone())
        })?;
        if canonical_build != build_directory
            || environment.build_directory.value() != Some(&canonical_build)
        {
            return Err(CapabilityProbeContextError::EnvironmentMismatch);
        }
        if tools.len() > MAX_PROBE_CONTEXT_VALUES
            || process_environment.len() > MAX_PROBE_ENVIRONMENT_VALUES
            || [
                metadata_tasks.as_ref().map_or(0, BTreeSet::len),
                metadata_variables.as_ref().map_or(0, BTreeSet::len),
                backend_capabilities.as_ref().map_or(0, BTreeSet::len),
                protocol_capabilities.as_ref().map_or(0, BTreeSet::len),
                artifacts.as_ref().map_or(0, BTreeSet::len),
                configurations.as_ref().map_or(0, BTreeSet::len),
            ]
            .into_iter()
            .any(|count| count > MAX_PROBE_CONTEXT_VALUES)
        {
            return Err(CapabilityProbeContextError::Oversized);
        }
        if process_environment
            .iter()
            .any(|(key, value)| !valid_environment_text(key) || !valid_environment_text(value))
            || metadata_tasks
                .iter()
                .flatten()
                .chain(metadata_variables.iter().flatten())
                .chain(backend_capabilities.iter().flatten())
                .chain(protocol_capabilities.iter().flatten())
                .chain(artifacts.iter().flatten())
                .chain(configurations.iter().flatten())
                .any(|value| !valid_token(value))
        {
            return Err(CapabilityProbeContextError::InvalidInput);
        }
        validate_tools(&environment.available_tools, &tools)?;
        Ok(Self {
            environment,
            build_directory: canonical_build,
            tools,
            process_environment,
            metadata_tasks,
            metadata_variables,
            backend_capabilities,
            protocol_capabilities,
            artifacts,
            configurations,
        })
    }

    pub fn environment(&self) -> &YoctoEnvironmentIdentity {
        &self.environment
    }

    pub fn matches_environment(&self, environment: &YoctoEnvironmentIdentity) -> bool {
        environment
            .clone()
            .normalize()
            .is_ok_and(|identity| identity == self.environment)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityProbeContextError {
    #[error(transparent)]
    InvalidIdentity(#[from] yoctui_model::EnvironmentIdentityError),
    #[error("capability probe build directory is unsafe: {0}")]
    UnsafeBuildDirectory(PathBuf),
    #[error("capability probe context does not match the exact environment identity")]
    EnvironmentMismatch,
    #[error("capability probe context is oversized")]
    Oversized,
    #[error("capability probe context contains invalid input")]
    InvalidInput,
    #[error("capability probe tool identity is unsafe: {0}")]
    UnsafeTool(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityProbeStatus {
    Positive,
    Negative,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityProbeObservation {
    pub status: CapabilityProbeStatus,
    pub evidence: CapabilityEvidence,
}

#[derive(Debug, Clone)]
pub struct CapabilityProbeRunner {
    timeout: Duration,
    output_limit_per_stream: usize,
    background_priority: bool,
}

impl Default for CapabilityProbeRunner {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_PROBE_TIMEOUT,
            output_limit_per_stream: DEFAULT_PROBE_OUTPUT_LIMIT,
            background_priority: false,
        }
    }
}

impl CapabilityProbeRunner {
    pub fn with_limits(
        timeout: Duration,
        output_limit_per_stream: usize,
    ) -> Result<Self, CapabilityProbeContextError> {
        if timeout.is_zero()
            || output_limit_per_stream == 0
            || output_limit_per_stream > MAX_PROBE_OUTPUT_LIMIT
        {
            return Err(CapabilityProbeContextError::InvalidInput);
        }
        Ok(Self {
            timeout,
            output_limit_per_stream,
            background_priority: false,
        })
    }

    /// Lower only the read-only probe children so interactive callers retain
    /// scheduler precedence while capability discovery is running.
    pub fn with_background_priority(mut self) -> Self {
        self.background_priority = true;
        self
    }

    pub async fn probe(
        &self,
        context: &CapabilityProbeContext,
        probe: &CapabilityProbeSpec,
    ) -> CapabilityProbeObservation {
        match probe {
            CapabilityProbeSpec::Executable { tool } => self.probe_executable(context, *tool),
            CapabilityProbeSpec::CommandVersion { tool } => {
                self.probe_command(context, *tool, vec!["--version".into()], None, true)
                    .await
            }
            CapabilityProbeSpec::CommandHelp { tool, subcommand } => {
                let mut argv = subcommand.iter().cloned().collect::<Vec<_>>();
                argv.push("--help".into());
                self.probe_command(context, *tool, argv, subcommand.as_deref(), false)
                    .await
            }
            CapabilityProbeSpec::CommandOption {
                tool,
                subcommand,
                option,
            } => {
                let mut argv = subcommand.iter().cloned().collect::<Vec<_>>();
                argv.push("--help".into());
                self.probe_command(context, *tool, argv, Some(option), false)
                    .await
            }
            CapabilityProbeSpec::CommandHelpText { tool, needle } => {
                self.probe_command(context, *tool, vec!["--help".into()], Some(needle), false)
                    .await
            }
            CapabilityProbeSpec::MetadataAnyTask { names } => observation(
                context.metadata_tasks.as_ref().map_or(
                    CapabilityProbeStatus::Inconclusive,
                    |tasks| {
                        if names.iter().any(|name| tasks.contains(name)) {
                            CapabilityProbeStatus::Positive
                        } else {
                            CapabilityProbeStatus::Negative
                        }
                    },
                ),
                CapabilityEvidenceKind::Metadata,
                "BitBake task inventory",
                context.metadata_tasks.as_ref().map_or_else(
                    || format!("Task inventory was not probed: {}", names.join(", ")),
                    |tasks| {
                        if names.iter().any(|name| tasks.contains(name)) {
                            format!(
                                "At least one required task is present: {}",
                                names.join(", ")
                            )
                        } else {
                            format!("No required task is present: {}", names.join(", "))
                        }
                    },
                ),
                Vec::new(),
            ),
            CapabilityProbeSpec::MetadataVariable { name } => set_observation(
                &context.metadata_variables,
                name,
                CapabilityEvidenceKind::Metadata,
                "BitBake metadata variable",
            ),
            CapabilityProbeSpec::BackendCapability { name } => set_observation(
                &context.backend_capabilities,
                name,
                CapabilityEvidenceKind::BackendNegotiation,
                "BitBake backend capability",
            ),
            CapabilityProbeSpec::ProtocolCapability { name } => set_observation(
                &context.protocol_capabilities,
                name,
                CapabilityEvidenceKind::ProtocolNegotiation,
                "Yoctui protocol capability",
            ),
            CapabilityProbeSpec::Artifact { kind } => set_observation(
                &context.artifacts,
                kind,
                CapabilityEvidenceKind::Metadata,
                "Build artifact inventory",
            ),
            CapabilityProbeSpec::Configuration { name } => set_observation(
                &context.configurations,
                name,
                CapabilityEvidenceKind::Metadata,
                "Build configuration",
            ),
        }
    }

    fn probe_executable(
        &self,
        context: &CapabilityProbeContext,
        tool: CapabilityToolId,
    ) -> CapabilityProbeObservation {
        let Some(path) = context.tools.get(&tool) else {
            return observation(
                CapabilityProbeStatus::Negative,
                CapabilityEvidenceKind::ExecutableIdentity,
                tool.executable_name(),
                format!(
                    "{} is absent from the initialized environment",
                    tool.executable_name()
                ),
                Vec::new(),
            );
        };
        match safe_executable(path) {
            Some(path) => observation(
                CapabilityProbeStatus::Positive,
                CapabilityEvidenceKind::ExecutableIdentity,
                tool.executable_name(),
                format!("Canonical executable is available at {}", path.display()),
                vec![path.display().to_string()],
            ),
            None => observation(
                CapabilityProbeStatus::Inconclusive,
                CapabilityEvidenceKind::ExecutableIdentity,
                tool.executable_name(),
                format!(
                    "Configured executable identity is unsafe or stale: {}",
                    path.display()
                ),
                vec![path.display().to_string()],
            ),
        }
    }

    async fn probe_command(
        &self,
        context: &CapabilityProbeContext,
        tool: CapabilityToolId,
        arguments: Vec<String>,
        expected: Option<&str>,
        require_output: bool,
    ) -> CapabilityProbeObservation {
        let Some(path) = context
            .tools
            .get(&tool)
            .and_then(|path| safe_executable(path))
        else {
            return self.probe_executable(context, tool);
        };
        let mut indexed = vec![path.display().to_string()];
        indexed.extend(arguments.iter().cloned());
        let result = run_read_only(
            &path,
            &arguments,
            &context.build_directory,
            &context.process_environment,
            self.timeout,
            self.output_limit_per_stream,
            self.background_priority,
        )
        .await;
        match result {
            ProbeProcessResult::Completed {
                success: _,
                output: _,
                truncated,
                ..
            } if truncated => observation(
                CapabilityProbeStatus::Inconclusive,
                CapabilityEvidenceKind::DirectProbe,
                tool.executable_name(),
                "Probe output exceeded its safety bound".into(),
                indexed,
            ),
            ProbeProcessResult::Completed { success: false, .. } => observation(
                CapabilityProbeStatus::Negative,
                CapabilityEvidenceKind::DirectProbe,
                tool.executable_name(),
                "Read-only help/version probe returned a non-zero status".into(),
                indexed,
            ),
            ProbeProcessResult::Completed { output, .. } => {
                let found = expected.is_none_or(|needle| output.contains(needle));
                let status = if found && (!require_output || !output.trim().is_empty()) {
                    CapabilityProbeStatus::Positive
                } else if require_output {
                    CapabilityProbeStatus::Inconclusive
                } else {
                    CapabilityProbeStatus::Negative
                };
                observation(
                    status,
                    CapabilityEvidenceKind::DirectProbe,
                    tool.executable_name(),
                    match (expected, status) {
                        (Some(value), CapabilityProbeStatus::Positive) => {
                            format!("Bounded help output exposes {value}")
                        }
                        (Some(value), CapabilityProbeStatus::Negative) => {
                            format!("Bounded help output does not expose {value}")
                        }
                        _ if status == CapabilityProbeStatus::Positive => {
                            "Bounded version/help probe succeeded".into()
                        }
                        _ => "Probe succeeded without authoritative output".into(),
                    },
                    indexed,
                )
            }
            ProbeProcessResult::TimedOut => observation(
                CapabilityProbeStatus::Inconclusive,
                CapabilityEvidenceKind::DirectProbe,
                tool.executable_name(),
                "Read-only probe timed out and its process group was terminated".into(),
                indexed,
            ),
            ProbeProcessResult::Failed(message) => observation(
                CapabilityProbeStatus::Inconclusive,
                CapabilityEvidenceKind::DirectProbe,
                tool.executable_name(),
                format!("Read-only probe failed: {}", bounded(&message)),
                indexed,
            ),
        }
    }
}
