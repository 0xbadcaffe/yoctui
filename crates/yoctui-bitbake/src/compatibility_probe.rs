use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use thiserror::Error;
use tokio::{io::AsyncReadExt, process::Command};
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

#[derive(Debug, Clone)]
pub struct CapabilityProbeContext {
    environment: YoctoEnvironmentIdentity,
    build_directory: PathBuf,
    tools: BTreeMap<CapabilityToolId, PathBuf>,
    process_environment: BTreeMap<String, String>,
    metadata_tasks: BTreeSet<String>,
    metadata_variables: BTreeSet<String>,
    backend_capabilities: BTreeSet<String>,
    protocol_capabilities: BTreeSet<String>,
    artifacts: BTreeSet<String>,
    configurations: BTreeSet<String>,
}

impl CapabilityProbeContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        environment: YoctoEnvironmentIdentity,
        build_directory: PathBuf,
        tools: BTreeMap<CapabilityToolId, PathBuf>,
        process_environment: BTreeMap<String, String>,
        metadata_tasks: BTreeSet<String>,
        metadata_variables: BTreeSet<String>,
        backend_capabilities: BTreeSet<String>,
        protocol_capabilities: BTreeSet<String>,
        artifacts: BTreeSet<String>,
        configurations: BTreeSet<String>,
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
                metadata_tasks.len(),
                metadata_variables.len(),
                backend_capabilities.len(),
                protocol_capabilities.len(),
                artifacts.len(),
                configurations.len(),
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
                .chain(metadata_variables.iter())
                .chain(backend_capabilities.iter())
                .chain(protocol_capabilities.iter())
                .chain(artifacts.iter())
                .chain(configurations.iter())
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
}

impl Default for CapabilityProbeRunner {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_PROBE_TIMEOUT,
            output_limit_per_stream: DEFAULT_PROBE_OUTPUT_LIMIT,
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
        })
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
            CapabilityProbeSpec::MetadataAnyTask { names } => observation(
                if names
                    .iter()
                    .any(|name| context.metadata_tasks.contains(name))
                {
                    CapabilityProbeStatus::Positive
                } else {
                    CapabilityProbeStatus::Negative
                },
                CapabilityEvidenceKind::Metadata,
                "BitBake task inventory",
                if names
                    .iter()
                    .any(|name| context.metadata_tasks.contains(name))
                {
                    format!(
                        "At least one required task is present: {}",
                        names.join(", ")
                    )
                } else {
                    format!("No required task is present: {}", names.join(", "))
                },
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
        )
        .await;
        match result {
            ProbeProcessResult::Completed {
                success: _,
                output: _,
                truncated,
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

enum ProbeProcessResult {
    Completed {
        success: bool,
        output: String,
        truncated: bool,
    },
    TimedOut,
    Failed(String),
}

async fn run_read_only(
    executable: &Path,
    arguments: &[String],
    cwd: &Path,
    environment: &BTreeMap<String, String>,
    timeout: Duration,
    output_limit: usize,
) -> ProbeProcessResult {
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .current_dir(cwd)
        .env_clear()
        .envs(environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return ProbeProcessResult::Failed(error.to_string()),
    };
    let process_group = child.id().map(|id| id as i32);
    let Some(stdout) = child.stdout.take() else {
        return ProbeProcessResult::Failed("stdout pipe is unavailable".into());
    };
    let Some(stderr) = child.stderr.take() else {
        return ProbeProcessResult::Failed("stderr pipe is unavailable".into());
    };
    let read = async {
        let mut stdout_bytes = Vec::new();
        let mut stderr_bytes = Vec::new();
        let mut bounded_stdout = stdout.take(output_limit as u64 + 1);
        let mut bounded_stderr = stderr.take(output_limit as u64 + 1);
        let stdout_read = bounded_stdout.read_to_end(&mut stdout_bytes);
        let stderr_read = bounded_stderr.read_to_end(&mut stderr_bytes);
        let (stdout_result, stderr_result, status) =
            tokio::join!(stdout_read, stderr_read, child.wait());
        stdout_result.map_err(|error| error.to_string())?;
        stderr_result.map_err(|error| error.to_string())?;
        let status = status.map_err(|error| error.to_string())?;
        let truncated = stdout_bytes.len() > output_limit || stderr_bytes.len() > output_limit;
        stdout_bytes.truncate(output_limit);
        stderr_bytes.truncate(output_limit);
        let output = format!(
            "{}\n{}",
            String::from_utf8_lossy(&stdout_bytes),
            String::from_utf8_lossy(&stderr_bytes)
        );
        Ok::<_, String>((status.success(), output, truncated))
    };
    match tokio::time::timeout(timeout, read).await {
        Ok(Ok((success, output, truncated))) => ProbeProcessResult::Completed {
            success,
            output,
            truncated,
        },
        Ok(Err(message)) => ProbeProcessResult::Failed(message),
        Err(_) => {
            #[cfg(unix)]
            if let Some(group) = process_group {
                // SAFETY: the child was placed in a new process group owned by this probe.
                let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
            }
            let _ = child.kill().await;
            let _ = child.wait().await;
            ProbeProcessResult::TimedOut
        }
    }
}

fn set_observation(
    values: &BTreeSet<String>,
    value: &str,
    kind: CapabilityEvidenceKind,
    subject: &str,
) -> CapabilityProbeObservation {
    let present = values.contains(value);
    observation(
        if present {
            CapabilityProbeStatus::Positive
        } else {
            CapabilityProbeStatus::Negative
        },
        kind,
        subject,
        if present {
            format!("Connected environment reports {value}")
        } else {
            format!("Connected environment does not report {value}")
        },
        Vec::new(),
    )
}

fn observation(
    status: CapabilityProbeStatus,
    kind: CapabilityEvidenceKind,
    subject: impl Into<String>,
    detail: String,
    argv: Vec<String>,
) -> CapabilityProbeObservation {
    CapabilityProbeObservation {
        status,
        evidence: CapabilityEvidence {
            kind,
            outcome: match status {
                CapabilityProbeStatus::Positive => CapabilityEvidenceOutcome::Positive,
                CapabilityProbeStatus::Negative => CapabilityEvidenceOutcome::Negative,
                CapabilityProbeStatus::Inconclusive => CapabilityEvidenceOutcome::Inconclusive,
            },
            subject: bounded(&subject.into()),
            detail: bounded(&detail),
            argv: argv.into_iter().map(|value| bounded(&value)).collect(),
        },
    }
}

fn validate_tools(
    identity: &AuthoritativeValue<Vec<ToolIdentity>>,
    tools: &BTreeMap<CapabilityToolId, PathBuf>,
) -> Result<(), CapabilityProbeContextError> {
    if tools.is_empty() {
        return Ok(());
    }
    let Some(authoritative) = identity.value() else {
        return Err(CapabilityProbeContextError::EnvironmentMismatch);
    };
    for (tool, path) in tools {
        if !valid_absolute_path(path) {
            return Err(CapabilityProbeContextError::UnsafeTool(path.clone()));
        }
        if !authoritative
            .iter()
            .any(|identity| identity.id == tool.executable_name() && identity.executable == *path)
        {
            return Err(CapabilityProbeContextError::EnvironmentMismatch);
        }
    }
    Ok(())
}

fn canonical_directory(path: &Path) -> Option<PathBuf> {
    if !valid_absolute_path(path) {
        return None;
    }
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    let canonical = fs::canonicalize(path).ok()?;
    (canonical == path).then_some(canonical)
}

fn safe_executable(path: &Path) -> Option<PathBuf> {
    if !valid_absolute_path(path) {
        return None;
    }
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }
    let canonical = fs::canonicalize(path).ok()?;
    (canonical == path).then_some(canonical)
}

fn valid_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && path.as_os_str().as_encoded_bytes().len() <= MAX_PROBE_TEXT_BYTES
        && path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
}

fn valid_environment_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROBE_TEXT_BYTES
        && !value.chars().any(|character| character == '\0')
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROBE_TEXT_BYTES
        && !value.chars().any(char::is_whitespace)
        && !value.chars().any(char::is_control)
}

fn bounded(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control() || *character == '\t')
        .take(MAX_PROBE_TEXT_BYTES)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::write_executable;
    use std::{
        env,
        sync::atomic::{AtomicU64, Ordering},
    };
    use yoctui_model::{IdentityAuthority, ToolIdentity};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    struct Fixture {
        root: PathBuf,
        tool: PathBuf,
    }

    impl Fixture {
        fn new(body: &str) -> Self {
            Self::new_tool(CapabilityToolId::Devtool, body)
        }

        fn new_tool(tool_id: CapabilityToolId, body: &str) -> Self {
            let root = env::temp_dir().join(format!(
                "yoctui-compat-probe-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            let tool = root.join(tool_id.executable_name());
            write_executable(&tool, body);
            Self { root, tool }
        }

        fn context(&self) -> CapabilityProbeContext {
            self.context_for(CapabilityToolId::Devtool)
        }

        fn context_for(&self, tool_id: CapabilityToolId) -> CapabilityProbeContext {
            let identity = YoctoEnvironmentIdentity {
                build_directory: AuthoritativeValue::detected(
                    self.root.clone(),
                    IdentityAuthority::InitializedEnvironment,
                ),
                available_tools: AuthoritativeValue::detected(
                    vec![ToolIdentity {
                        id: tool_id.executable_name().into(),
                        executable: self.tool.clone(),
                        version: None,
                    }],
                    IdentityAuthority::ExecutableProbe,
                ),
                ..YoctoEnvironmentIdentity::default()
            };
            CapabilityProbeContext::new(
                identity,
                self.root.clone(),
                BTreeMap::from([(tool_id, self.tool.clone())]),
                BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
                BTreeSet::from(["create_spdx".into()]),
                BTreeSet::from(["MACHINE".into()]),
                BTreeSet::from(["getvar".into()]),
                BTreeSet::from(["state_snapshots".into()]),
                BTreeSet::from(["wic".into()]),
                BTreeSet::from(["buildhistory".into()]),
            )
            .unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[tokio::test]
    async fn compatibility_probe_uses_exact_shell_free_help_and_version_argv() {
        let fixture = Fixture::new(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> probe.argv\ncase \"$*\" in\n  *--version*) echo 'devtool 1.0' ;;\n  *) echo 'modify upgrade --force' ;;\nesac\n",
        );
        let context = fixture.context();
        let runner = CapabilityProbeRunner::default();
        let help = runner
            .probe(
                &context,
                &CapabilityProbeSpec::CommandHelp {
                    tool: CapabilityToolId::Devtool,
                    subcommand: Some("upgrade".into()),
                },
            )
            .await;
        assert_eq!(help.status, CapabilityProbeStatus::Positive);
        assert_eq!(help.evidence.argv[1..], ["upgrade", "--help"]);
        let version = runner
            .probe(
                &context,
                &CapabilityProbeSpec::CommandVersion {
                    tool: CapabilityToolId::Devtool,
                },
            )
            .await;
        assert_eq!(version.status, CapabilityProbeStatus::Positive);
        assert_eq!(version.evidence.argv[1..], ["--version"]);
        assert_eq!(
            fs::read_to_string(fixture.root.join("probe.argv")).unwrap(),
            "upgrade\n--help\n--version\n"
        );
    }

    #[tokio::test]
    async fn compatibility_probe_distinguishes_missing_tool_command_and_option() {
        let fixture =
            Fixture::new("#!/bin/sh\nif [ \"$1\" = bad ]; then exit 2; fi\necho 'modify finish'\n");
        let context = fixture.context();
        let runner = CapabilityProbeRunner::default();
        let missing_tool = runner
            .probe(
                &context,
                &CapabilityProbeSpec::Executable {
                    tool: CapabilityToolId::Wic,
                },
            )
            .await;
        assert_eq!(missing_tool.status, CapabilityProbeStatus::Negative);
        let command = runner
            .probe(
                &context,
                &CapabilityProbeSpec::CommandHelp {
                    tool: CapabilityToolId::Devtool,
                    subcommand: Some("bad".into()),
                },
            )
            .await;
        assert_eq!(command.status, CapabilityProbeStatus::Negative);
        let option = runner
            .probe(
                &context,
                &CapabilityProbeSpec::CommandOption {
                    tool: CapabilityToolId::Devtool,
                    subcommand: None,
                    option: "--force".into(),
                },
            )
            .await;
        assert_eq!(option.status, CapabilityProbeStatus::Negative);
    }

    #[tokio::test]
    async fn compatibility_probe_bitbake_getvar_uses_exact_initialized_tool_and_options() {
        let fixture = Fixture::new_tool(
            CapabilityToolId::BitBakeGetVar,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> probe.argv\necho 'usage: bitbake-getvar [--value] [-r RECIPE] variable'\necho '  -r, --recipe RECIPE'\n",
        );
        let context = fixture.context_for(CapabilityToolId::BitBakeGetVar);
        let runner = CapabilityProbeRunner::default();
        for probe in [
            CapabilityProbeSpec::Executable {
                tool: CapabilityToolId::BitBakeGetVar,
            },
            CapabilityProbeSpec::CommandHelp {
                tool: CapabilityToolId::BitBakeGetVar,
                subcommand: None,
            },
            CapabilityProbeSpec::CommandOption {
                tool: CapabilityToolId::BitBakeGetVar,
                subcommand: None,
                option: "--value".into(),
            },
            CapabilityProbeSpec::CommandOption {
                tool: CapabilityToolId::BitBakeGetVar,
                subcommand: None,
                option: "--recipe".into(),
            },
        ] {
            assert_eq!(
                runner.probe(&context, &probe).await.status,
                CapabilityProbeStatus::Positive
            );
        }
        assert_eq!(
            fs::read_to_string(fixture.root.join("probe.argv")).unwrap(),
            "--help\n--help\n--help\n"
        );

        let missing_context = Fixture::new("#!/bin/sh\necho ok\n").context();
        assert_eq!(
            runner
                .probe(
                    &missing_context,
                    &CapabilityProbeSpec::Executable {
                        tool: CapabilityToolId::BitBakeGetVar
                    }
                )
                .await
                .status,
            CapabilityProbeStatus::Negative
        );
    }

    #[tokio::test]
    async fn compatibility_probe_reports_timeout_oversize_and_stale_executable_as_inconclusive() {
        let timeout_fixture = Fixture::new("#!/bin/sh\nsleep 30\n");
        let runner = CapabilityProbeRunner::with_limits(Duration::from_millis(30), 128).unwrap();
        let timed_out = runner
            .probe(
                &timeout_fixture.context(),
                &CapabilityProbeSpec::CommandVersion {
                    tool: CapabilityToolId::Devtool,
                },
            )
            .await;
        assert_eq!(timed_out.status, CapabilityProbeStatus::Inconclusive);
        assert!(timed_out.evidence.detail.contains("timed out"));

        let large_fixture = Fixture::new("#!/bin/sh\nyes x | head -c 4096\n");
        let oversized = runner
            .probe(
                &large_fixture.context(),
                &CapabilityProbeSpec::CommandVersion {
                    tool: CapabilityToolId::Devtool,
                },
            )
            .await;
        assert_eq!(oversized.status, CapabilityProbeStatus::Inconclusive);
        assert!(oversized.evidence.detail.contains("safety bound"));

        let stale_fixture = Fixture::new("#!/bin/sh\necho ok\n");
        let context = stale_fixture.context();
        fs::remove_file(&stale_fixture.tool).unwrap();
        let stale = CapabilityProbeRunner::default()
            .probe(
                &context,
                &CapabilityProbeSpec::Executable {
                    tool: CapabilityToolId::Devtool,
                },
            )
            .await;
        assert_eq!(stale.status, CapabilityProbeStatus::Inconclusive);
    }

    #[tokio::test]
    async fn compatibility_probe_maps_typed_non_process_observations() {
        let fixture = Fixture::new("#!/bin/sh\necho ok\n");
        let context = fixture.context();
        let runner = CapabilityProbeRunner::default();
        for probe in [
            CapabilityProbeSpec::MetadataAnyTask {
                names: vec!["create_spdx".into()],
            },
            CapabilityProbeSpec::MetadataVariable {
                name: "MACHINE".into(),
            },
            CapabilityProbeSpec::BackendCapability {
                name: "getvar".into(),
            },
            CapabilityProbeSpec::ProtocolCapability {
                name: "state_snapshots".into(),
            },
            CapabilityProbeSpec::Artifact { kind: "wic".into() },
            CapabilityProbeSpec::Configuration {
                name: "buildhistory".into(),
            },
        ] {
            assert_eq!(
                runner.probe(&context, &probe).await.status,
                CapabilityProbeStatus::Positive
            );
        }
        assert_eq!(
            runner
                .probe(
                    &context,
                    &CapabilityProbeSpec::MetadataVariable {
                        name: "ABSENT".into()
                    },
                )
                .await
                .status,
            CapabilityProbeStatus::Negative
        );
    }

    #[test]
    fn compatibility_probe_context_rejects_environment_and_tool_mismatch() {
        let fixture = Fixture::new("#!/bin/sh\necho ok\n");
        let mut identity = fixture.context().environment().clone();
        identity.build_directory = AuthoritativeValue::detected(
            "/other/build".into(),
            IdentityAuthority::InitializedEnvironment,
        );
        let result = CapabilityProbeContext::new(
            identity,
            fixture.root.clone(),
            BTreeMap::from([(CapabilityToolId::Devtool, fixture.tool.clone())]),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        );
        assert!(matches!(
            result,
            Err(CapabilityProbeContextError::EnvironmentMismatch)
        ));

        let mut identity = fixture.context().environment().clone();
        identity.available_tools = AuthoritativeValue::detected(
            vec![ToolIdentity {
                id: "devtool".into(),
                executable: "/other/devtool".into(),
                version: None,
            }],
            IdentityAuthority::ExecutableProbe,
        );
        let result = CapabilityProbeContext::new(
            identity,
            fixture.root.clone(),
            BTreeMap::from([(CapabilityToolId::Devtool, fixture.tool.clone())]),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        );
        assert!(matches!(
            result,
            Err(CapabilityProbeContextError::EnvironmentMismatch)
        ));
    }
}
