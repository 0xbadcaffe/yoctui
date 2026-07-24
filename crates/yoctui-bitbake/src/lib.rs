//! BitBake adapters. They execute BitBake; they never evaluate metadata themselves.
use async_trait::async_trait;
use std::{
    ffi::OsString,
    path::PathBuf,
    process::Stdio,
    time::{Duration, SystemTime},
};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command as TokioCommand},
};
use yoctui_model::{BuildRequest, Layer, LogEntry, Recipe, Severity, Workspace};
use yoctui_protocol::{
    Command, Envelope, Event, LayerData, LayerRelationshipData, MAX_LINE_BYTES, ProtocolError,
    RecipeData, VERSION, decode_line, encode_line,
};

const MAX_PROCESS_LINE_BYTES: usize = 1024 * 1024;

async fn read_output<R>(stream: R, sender: tokio::sync::mpsc::Sender<LogEntry>)
where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut bytes = Vec::new();
    let mut discarding = false;
    while let Ok(buffer) = reader.fill_buf().await {
        if buffer.is_empty() {
            if !bytes.is_empty()
                && !discarding
                && sender
                    .send(classify_output(output_text(&bytes)))
                    .await
                    .is_err()
            {
                break;
            }
            break;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(buffer.len());
        if !discarding {
            if bytes.len() + take > MAX_PROCESS_LINE_BYTES {
                let mut message = output_text(&bytes);
                message.push_str(" [line truncated]");
                if sender.send(classify_output(message)).await.is_err() {
                    break;
                }
                bytes.clear();
                discarding = true;
            } else {
                bytes.extend_from_slice(&buffer[..take]);
            }
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if !discarding
                && sender
                    .send(classify_output(output_text(&bytes)))
                    .await
                    .is_err()
            {
                break;
            }
            bytes.clear();
            discarding = false;
        }
    }
}

pub fn output_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches(['\r', '\n'])
        .into()
}
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("process: {0}")]
    Process(#[from] std::io::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("bridge: {0}")]
    Bridge(String),
    #[error("backend is not running")]
    NotRunning,
}
#[derive(Debug, Clone)]
pub enum BackendEvent {
    Workspace(Workspace),
    Recipes(Vec<Recipe>),
    Layers(Vec<Layer>),
    Variable {
        name: String,
        value: Option<String>,
        provenance: Option<String>,
    },
    Dependencies {
        recipe: String,
        build: Vec<String>,
        runtime: Vec<String>,
    },
    RecipeSources {
        recipe: String,
        paths: Vec<PathBuf>,
    },
    LayerRelationships(Vec<LayerRelationship>),
    BuildStarted,
    ParseProgress {
        current: Option<u64>,
        total: Option<u64>,
    },
    Log(LogEntry),
    TaskStarted {
        recipe: String,
        task: String,
    },
    TaskProgress {
        recipe: String,
        task: String,
        progress: Option<u8>,
    },
    TaskCompleted {
        recipe: String,
        task: String,
        success: bool,
    },
    BuildCompleted {
        success: bool,
        exit_code: Option<i32>,
    },
    CommandFailed {
        code: String,
        message: String,
    },
    Ignored,
    Disconnected,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariableValue {
    pub value: Option<String>,
    pub provenance: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecipeDependencies {
    pub build: Vec<String>,
    pub runtime: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayerRelationship {
    pub name: String,
    pub priority: Option<i32>,
    pub compatible: Vec<String>,
    pub depends: Vec<String>,
    pub overlays: Vec<String>,
    pub appends: Vec<String>,
}

#[async_trait]
pub trait BitBakeBackend: Send {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError>;
    async fn list_recipes(&mut self, filter: Option<String>) -> Result<Vec<Recipe>, BackendError>;
    async fn list_layers(&mut self) -> Result<Vec<Layer>, BackendError>;
    async fn get_variable(
        &mut self,
        name: String,
        recipe: Option<String>,
    ) -> Result<VariableValue, BackendError>;
    async fn get_dependencies(
        &mut self,
        recipe: String,
    ) -> Result<RecipeDependencies, BackendError>;
    async fn get_recipe_sources(&mut self, recipe: String) -> Result<Vec<PathBuf>, BackendError>;
    async fn get_layer_relationships(&mut self) -> Result<Vec<LayerRelationship>, BackendError>;
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError>;
    async fn cancel_build(&mut self) -> Result<(), BackendError>;
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError>;
    async fn shutdown(&mut self) -> Result<(), BackendError>;
}
pub fn strip_ansi(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for x in chars.by_ref() {
                if x.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c)
        }
    }
    out
}
pub fn classify_output(line: String) -> LogEntry {
    let clean = strip_ansi(&line);
    let lower = clean.to_ascii_lowercase();
    let severity = if lower.contains("error:") || lower.starts_with("error") {
        Severity::Error
    } else if lower.contains("warning:") || lower.starts_with("warning") {
        Severity::Warning
    } else {
        Severity::Info
    };
    LogEntry {
        severity,
        message: clean,
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::now(),
    }
}
pub struct ProcessBackend {
    build_dir: PathBuf,
    executable: PathBuf,
    arguments: Vec<OsString>,
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<LogEntry>>,
    build_started_pending: bool,
    cancellation_timeout: Duration,
    #[cfg(unix)]
    process_group: Option<i32>,
}
impl ProcessBackend {
    pub fn new(build_dir: PathBuf) -> Self {
        Self::with_executable(build_dir, PathBuf::from("bitbake"))
    }

    pub fn with_executable(build_dir: PathBuf, executable: PathBuf) -> Self {
        Self::with_command(build_dir, executable, Vec::new())
    }

    pub fn with_command(build_dir: PathBuf, executable: PathBuf, arguments: Vec<OsString>) -> Self {
        Self {
            build_dir,
            executable,
            arguments,
            child: None,
            output: None,
            build_started_pending: false,
            cancellation_timeout: Duration::from_secs(5),
            #[cfg(unix)]
            process_group: None,
        }
    }
    pub fn with_cancellation_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_timeout = timeout;
        self
    }
    async fn collect(&mut self) -> Result<(bool, Option<i32>), BackendError> {
        let child = self.child.as_mut().ok_or(BackendError::NotRunning)?;
        let status = child.wait().await?;
        Ok((status.success(), status.code()))
    }
}
#[async_trait]
impl BitBakeBackend for ProcessBackend {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError> {
        Ok(Workspace {
            build_dir: Some(self.build_dir.clone()),
            ..Workspace::default()
        })
    }
    async fn list_recipes(&mut self, _filter: Option<String>) -> Result<Vec<Recipe>, BackendError> {
        Ok(Vec::new())
    }
    async fn list_layers(&mut self) -> Result<Vec<Layer>, BackendError> {
        Ok(Vec::new())
    }
    async fn get_variable(
        &mut self,
        _name: String,
        _recipe: Option<String>,
    ) -> Result<VariableValue, BackendError> {
        Ok(VariableValue::default())
    }
    async fn get_dependencies(
        &mut self,
        _recipe: String,
    ) -> Result<RecipeDependencies, BackendError> {
        Err(BackendError::Bridge(
            "the process backend cannot inspect authoritative recipe dependencies; use the Yoctui bridge"
                .into(),
        ))
    }
    async fn get_recipe_sources(&mut self, _recipe: String) -> Result<Vec<PathBuf>, BackendError> {
        Err(BackendError::Bridge("the process backend cannot inspect authoritative recipe source paths; use the Yoctui bridge".into()))
    }
    async fn get_layer_relationships(&mut self) -> Result<Vec<LayerRelationship>, BackendError> {
        Err(BackendError::Bridge("the process backend cannot inspect authoritative layer relationships; use the Yoctui bridge".into()))
    }
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError> {
        request
            .validate()
            .map_err(|e| BackendError::Bridge(e.to_string()))?;
        let mut cmd = TokioCommand::new(&self.executable);
        cmd.args(&self.arguments)
            .args(&request.targets)
            .current_dir(&self.build_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        cmd.process_group(0);
        let mut child = cmd.spawn()?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let stdout = child
            .stdout
            .take()
            .ok_or(BackendError::Bridge("stdout unavailable".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or(BackendError::Bridge("stderr unavailable".into()))?;
        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        tokio::spawn(read_output(stdout, tx.clone()));
        tokio::spawn(read_output(stderr, tx.clone()));
        drop(tx);
        self.child = Some(child);
        self.output = Some(rx);
        self.build_started_pending = true;
        Ok(())
    }
    async fn cancel_build(&mut self) -> Result<(), BackendError> {
        let c = self.child.as_mut().ok_or(BackendError::NotRunning)?;
        #[cfg(unix)]
        if let Some(process_group) = self.process_group {
            // SAFETY: process_group comes from the child PID after `process_group(0)`, and a
            // negative PID targets only that child process group, never the caller's group.
            let result = unsafe { libc::kill(-process_group, libc::SIGTERM) };
            if result == 0
                && tokio::time::timeout(self.cancellation_timeout, c.wait())
                    .await
                    .is_ok()
            {
                return Ok(());
            }
            // SAFETY: same process-group identity and scope as the graceful signal above.
            let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
        }
        c.kill().await?;
        let _ = c.wait().await?;
        Ok(())
    }
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError> {
        if self.build_started_pending {
            self.build_started_pending = false;
            return Ok(BackendEvent::BuildStarted);
        }
        if let Some(output) = self.output.as_mut()
            && let Some(line) = output.recv().await
        {
            return Ok(BackendEvent::Log(line));
        }
        let (success, exit_code) = self.collect().await?;
        Ok(BackendEvent::BuildCompleted { success, exit_code })
    }

    async fn shutdown(&mut self) -> Result<(), BackendError> {
        if let Some(child) = self.child.as_mut()
            && child.try_wait()?.is_none()
        {
            self.cancel_build().await?;
        }
        Ok(())
    }
}
pub struct BridgeBackend {
    child: Child,
    stdin: ChildStdin,
    lines: BufReader<tokio::process::ChildStdout>,
    sequence: u64,
    last_sequence: u64,
}
impl BridgeBackend {
    pub async fn spawn(
        python: &str,
        script: PathBuf,
        build_dir: PathBuf,
    ) -> Result<Self, BackendError> {
        let mut child = TokioCommand::new(python)
            .arg(script)
            .current_dir(build_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| BackendError::Bridge("bridge stdin unavailable".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BackendError::Bridge("bridge stdout unavailable".into()))?;
        let mut backend = Self {
            child,
            stdin,
            lines: BufReader::new(stdout),
            sequence: 0,
            last_sequence: 0,
        };
        backend.handshake().await?;
        Ok(backend)
    }
    async fn command(&mut self, message: Command) -> Result<(), BackendError> {
        self.sequence += 1;
        let bytes = encode_line(&Envelope {
            protocol_version: VERSION,
            sequence: self.sequence,
            correlation_id: Some(self.sequence.to_string()),
            message,
        })?;
        self.stdin.write_all(&bytes).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn next_line(&mut self) -> Result<Option<Vec<u8>>, BackendError> {
        let mut line = Vec::new();
        loop {
            let buffer = self.lines.fill_buf().await?;
            if buffer.is_empty() {
                return if line.is_empty() {
                    Ok(None)
                } else {
                    Err(ProtocolError::TooLarge.into())
                };
            }
            let newline = buffer.iter().position(|byte| *byte == b'\n');
            let take = newline.unwrap_or(buffer.len());
            if line.len() + take > MAX_LINE_BYTES {
                self.lines.consume(take);
                return Err(ProtocolError::TooLarge.into());
            }
            line.extend_from_slice(&buffer[..take]);
            self.lines.consume(take + usize::from(newline.is_some()));
            if newline.is_some() {
                return Ok(Some(line));
            }
        }
    }

    async fn handshake(&mut self) -> Result<(), BackendError> {
        self.command(Command::Hello).await?;
        let Some(line) = self.next_line().await? else {
            return Err(BackendError::Bridge(
                "bridge disconnected during protocol handshake".into(),
            ));
        };
        let envelope: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
        self.last_sequence = envelope.sequence;
        match envelope.message {
            Event::HelloAck { .. } => Ok(()),
            Event::ProtocolError { code, message } | Event::CommandFailed { code, message } => Err(
                BackendError::Bridge(format!("handshake rejected: {code}: {message}")),
            ),
            _ => Err(BackendError::Bridge(
                "bridge sent an unexpected handshake event".into(),
            )),
        }
    }

    /// Ask the bridge to finish its protocol work before the drop fallback kills it.
    pub async fn shutdown(&mut self) -> Result<(), BackendError> {
        self.command(Command::Shutdown).await?;
        let Some(line) = self.next_line().await? else {
            return Err(BackendError::Bridge(
                "bridge disconnected before acknowledging shutdown".into(),
            ));
        };
        let envelope: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
        self.last_sequence = envelope.sequence;
        match envelope.message {
            Event::BridgeShutdown => {}
            Event::CommandFailed { code, message } | Event::ProtocolError { code, message } => {
                return Err(BackendError::Bridge(format!(
                    "shutdown rejected: {code}: {message}"
                )));
            }
            _ => {
                return Err(BackendError::Bridge(
                    "bridge sent an unexpected shutdown event".into(),
                ));
            }
        }
        tokio::time::timeout(Duration::from_secs(2), self.child.wait())
            .await
            .map_err(|_| {
                BackendError::Bridge("bridge did not exit after shutdown acknowledgement".into())
            })??;
        Ok(())
    }
    fn event(event: Event) -> Result<BackendEvent, BackendError> {
        Ok(match event {
            Event::Workspace { data } => BackendEvent::Workspace(Workspace {
                build_dir: data.build_dir.map(PathBuf::from),
                source_dir: data.source_dir.map(PathBuf::from),
                variables: data.variables,
                variable_provenance: data.variable_provenance,
                variable_provenance_chain: data.variable_provenance_chain,
                bitbake_version: data.bitbake_version,
                release: data.release,
                layers: data
                    .layers
                    .into_iter()
                    .map(|layer| Layer {
                        name: layer.name,
                        path: PathBuf::from(layer.path),
                        priority: layer.priority,
                    })
                    .collect(),
                recipes: data
                    .recipes
                    .into_iter()
                    .map(|recipe| Recipe {
                        name: recipe.name,
                        version: recipe.version,
                        layer: recipe.layer,
                    })
                    .collect(),
            }),
            Event::Recipes { recipes } => BackendEvent::Recipes(
                recipes
                    .into_iter()
                    .map(
                        |RecipeData {
                             name,
                             version,
                             layer,
                         }| Recipe {
                            name,
                            version,
                            layer,
                        },
                    )
                    .collect(),
            ),
            Event::Layers { layers } => BackendEvent::Layers(
                layers
                    .into_iter()
                    .map(
                        |LayerData {
                             name,
                             path,
                             priority,
                         }| Layer {
                            name,
                            path: PathBuf::from(path),
                            priority,
                        },
                    )
                    .collect(),
            ),
            Event::Variable {
                name,
                value,
                provenance,
            } => BackendEvent::Variable {
                name,
                value,
                provenance,
            },
            Event::Dependencies {
                recipe,
                build,
                runtime,
            } => BackendEvent::Dependencies {
                recipe,
                build,
                runtime,
            },
            Event::RecipeSources { recipe, paths } => BackendEvent::RecipeSources {
                recipe,
                paths: paths.into_iter().map(PathBuf::from).collect(),
            },
            Event::LayerRelationships { layers } => BackendEvent::LayerRelationships(
                layers
                    .into_iter()
                    .map(
                        |LayerRelationshipData {
                             name,
                             priority,
                             compatible,
                             depends,
                             overlays,
                             appends,
                         }| LayerRelationship {
                            name,
                            priority,
                            compatible,
                            depends,
                            overlays,
                            appends,
                        },
                    )
                    .collect(),
            ),
            Event::BuildStarted => BackendEvent::BuildStarted,
            Event::ParseProgress { current, total } => {
                BackendEvent::ParseProgress { current, total }
            }
            Event::TaskStarted { recipe, task, .. } => BackendEvent::TaskStarted { recipe, task },
            Event::TaskProgress {
                recipe,
                task,
                progress,
            } => BackendEvent::TaskProgress {
                recipe,
                task,
                progress,
            },
            Event::TaskCompleted {
                recipe,
                task,
                success,
            } => BackendEvent::TaskCompleted {
                recipe,
                task,
                success,
            },
            Event::Log {
                level,
                message,
                recipe,
                task,
                path,
            } => {
                let severity = match level.as_str() {
                    "warning" => Severity::Warning,
                    "error" => Severity::Error,
                    _ => Severity::Info,
                };
                BackendEvent::Log(LogEntry {
                    severity,
                    message,
                    recipe,
                    task,
                    path: path.map(PathBuf::from),
                    timestamp: SystemTime::now(),
                })
            }
            Event::Warning { message } => BackendEvent::Log(LogEntry {
                severity: Severity::Warning,
                message,
                recipe: None,
                task: None,
                path: None,
                timestamp: SystemTime::now(),
            }),
            Event::Error { message } => BackendEvent::Log(LogEntry {
                severity: Severity::Error,
                message,
                recipe: None,
                task: None,
                path: None,
                timestamp: SystemTime::now(),
            }),
            Event::BuildCompleted { success, exit_code } => {
                BackendEvent::BuildCompleted { success, exit_code }
            }
            Event::CommandFailed { code, message } | Event::ProtocolError { code, message } => {
                BackendEvent::CommandFailed { code, message }
            }
            Event::BridgeShutdown => BackendEvent::Disconnected,
            Event::HelloAck { .. } | Event::Unknown => BackendEvent::Ignored,
        })
    }
}
#[async_trait]
impl BitBakeBackend for BridgeBackend {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError> {
        self.command(Command::InspectWorkspace).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Workspace(workspace) => return Ok(workspace),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(BackendError::Bridge(
                        "bridge disconnected during inspection".into(),
                    ));
                }
                _ => {}
            }
        }
    }
    async fn list_recipes(&mut self, filter: Option<String>) -> Result<Vec<Recipe>, BackendError> {
        self.command(Command::ListRecipes { filter }).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Recipes(recipes) => return Ok(recipes),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(BackendError::Bridge(
                        "bridge disconnected while listing recipes".into(),
                    ));
                }
                _ => {}
            }
        }
    }
    async fn list_layers(&mut self) -> Result<Vec<Layer>, BackendError> {
        self.command(Command::ListLayers).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Layers(layers) => return Ok(layers),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(BackendError::Bridge(
                        "bridge disconnected while listing layers".into(),
                    ));
                }
                _ => {}
            }
        }
    }
    async fn get_variable(
        &mut self,
        name: String,
        recipe: Option<String>,
    ) -> Result<VariableValue, BackendError> {
        self.command(Command::GetVariable {
            name: name.clone(),
            recipe,
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Variable {
                    name: returned,
                    value,
                    provenance,
                } if returned == name => return Ok(VariableValue { value, provenance }),
                BackendEvent::Variable { .. } => continue,
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(BackendError::Bridge(
                        "bridge disconnected while reading a variable".into(),
                    ));
                }
                _ => {}
            }
        }
    }
    async fn get_dependencies(
        &mut self,
        recipe: String,
    ) -> Result<RecipeDependencies, BackendError> {
        self.command(Command::GetDependencies {
            recipe: recipe.clone(),
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Dependencies {
                    recipe: returned,
                    build,
                    runtime,
                } if returned == recipe => return Ok(RecipeDependencies { build, runtime }),
                BackendEvent::Dependencies { .. } => continue,
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(BackendError::Bridge(
                        "bridge disconnected while reading recipe dependencies".into(),
                    ));
                }
                _ => continue,
            }
        }
    }
    async fn get_recipe_sources(&mut self, recipe: String) -> Result<Vec<PathBuf>, BackendError> {
        self.command(Command::GetRecipeSources {
            recipe: recipe.clone(),
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::RecipeSources {
                    recipe: returned,
                    paths,
                } if returned == recipe => return Ok(paths),
                BackendEvent::RecipeSources { .. } => continue,
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(BackendError::Bridge(
                        "bridge disconnected while reading recipe source paths".into(),
                    ));
                }
                _ => continue,
            }
        }
    }
    async fn get_layer_relationships(&mut self) -> Result<Vec<LayerRelationship>, BackendError> {
        self.command(Command::GetLayerRelationships).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::LayerRelationships(layers) => return Ok(layers),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(BackendError::Bridge(
                        "bridge disconnected while reading layer relationships".into(),
                    ));
                }
                _ => continue,
            }
        }
    }
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError> {
        self.command(Command::StartBuild {
            targets: request.targets,
            task: request.task,
        })
        .await
    }
    async fn cancel_build(&mut self) -> Result<(), BackendError> {
        self.command(Command::CancelBuild).await
    }
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError> {
        let Some(line) = self.next_line().await? else {
            return Ok(BackendEvent::Disconnected);
        };
        let e: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
        self.last_sequence = e.sequence;
        Self::event(e.message)
    }

    async fn shutdown(&mut self) -> Result<(), BackendError> {
        BridgeBackend::shutdown(self).await
    }
}
impl Drop for BridgeBackend {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn fixture_script(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("yoctui-{name}-{}-{nonce}", std::process::id()))
    }

    fn shell_backend(script: PathBuf) -> ProcessBackend {
        ProcessBackend::with_command(
            std::env::temp_dir(),
            PathBuf::from("/bin/sh"),
            vec![script.into_os_string()],
        )
    }
    #[test]
    fn ansi_and_severity() {
        assert_eq!(strip_ansi("\x1b[31merror: bad\x1b[0m"), "error: bad");
        assert_eq!(
            classify_output("WARNING: x".into()).severity,
            Severity::Warning
        )
    }
    #[test]
    fn typed_event_preserves_unknown_progress_and_ignores_future_events() {
        assert!(matches!(
            BridgeBackend::event(Event::TaskProgress {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: None,
            })
            .unwrap(),
            BackendEvent::TaskProgress { progress: None, .. }
        ));
        assert!(matches!(
            BridgeBackend::event(Event::Unknown).unwrap(),
            BackendEvent::Ignored
        ));
        assert!(matches!(
            BridgeBackend::event(Event::BridgeShutdown).unwrap(),
            BackendEvent::Disconnected
        ));
    }

    #[test]
    fn typed_event_workspace_converts_wire_paths_and_metadata() {
        let event = Event::Workspace {
            data: yoctui_protocol::WorkspaceData {
                build_dir: Some("/build".into()),
                source_dir: Some("/poky".into()),
                variables: std::collections::HashMap::from([(
                    "MACHINE".into(),
                    "qemux86-64".into(),
                )]),
                variable_provenance: std::collections::HashMap::new(),
                variable_provenance_chain: std::collections::HashMap::new(),
                bitbake_version: Some("2.19.0".into()),
                release: Some("6.0".into()),
                layers: vec![LayerData {
                    name: "core".into(),
                    path: "/poky/meta".into(),
                    priority: Some(5),
                }],
                recipes: vec![RecipeData {
                    name: "base-files".into(),
                    version: None,
                    layer: Some("core".into()),
                }],
            },
        };
        let BackendEvent::Workspace(workspace) = BridgeBackend::event(event).unwrap() else {
            panic!("workspace event was not preserved");
        };
        assert_eq!(workspace.build_dir, Some(PathBuf::from("/build")));
        assert_eq!(workspace.layers[0].path, PathBuf::from("/poky/meta"));
        assert_eq!(workspace.recipes[0].name, "base-files");
    }
    #[test]
    fn invalid_utf8_output_is_preserved_lossily() {
        assert_eq!(output_text(b"warning: \xff\n"), "warning: �");
    }

    #[tokio::test]
    async fn oversized_process_line_is_truncated_and_stream_continues() {
        let (mut writer, reader) = tokio::io::duplex(MAX_PROCESS_LINE_BYTES + 2);
        let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
        let reader_task = tokio::spawn(read_output(reader, sender));
        writer
            .write_all(&vec![b'x'; MAX_PROCESS_LINE_BYTES + 1])
            .await
            .unwrap();
        writer.write_all(b"\nnext line\n").await.unwrap();
        drop(writer);
        reader_task.await.unwrap();
        assert!(
            receiver
                .recv()
                .await
                .unwrap()
                .message
                .ends_with("[line truncated]")
        );
        assert_eq!(receiver.recv().await.unwrap().message, "next line");
    }

    #[tokio::test]
    async fn process_backend_collects_both_output_streams() {
        let script = fixture_script("fake-bitbake");
        fs::write(
            &script,
            "#!/bin/sh\nprintf 'NOTE: stdout line\\n'\nprintf 'WARNING: stderr line\\n' >&2\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();
        let mut backend = shell_backend(script.clone());
        backend
            .start_build(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
            })
            .await
            .unwrap();
        let mut messages = Vec::new();
        loop {
            match backend.next_event().await.unwrap() {
                BackendEvent::Log(entry) => messages.push(entry),
                BackendEvent::BuildCompleted { success, .. } => {
                    assert!(success);
                    break;
                }
                _ => {}
            }
        }
        fs::remove_file(script).unwrap();
        assert_eq!(messages.len(), 2);
        assert!(
            messages
                .iter()
                .any(|entry| entry.severity == Severity::Warning)
        );
    }

    #[tokio::test]
    async fn process_backend_cancellation_acknowledges_a_hung_child() {
        let script = fixture_script("hung-bitbake");
        fs::write(&script, "#!/bin/sh\nsleep 30\n").unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();
        let mut backend = shell_backend(script.clone());
        backend
            .start_build(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
            })
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), backend.cancel_build())
            .await
            .unwrap()
            .unwrap();
        loop {
            if let BackendEvent::BuildCompleted { success, .. } =
                tokio::time::timeout(Duration::from_secs(2), backend.next_event())
                    .await
                    .unwrap()
                    .unwrap()
            {
                assert!(!success);
                break;
            }
        }
        fs::remove_file(script).unwrap();
    }

    #[tokio::test]
    async fn process_backend_escalates_after_configured_cancellation_timeout() {
        let script = fixture_script("term-ignoring-bitbake");
        fs::write(
            &script,
            "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();
        let mut backend =
            shell_backend(script.clone()).with_cancellation_timeout(Duration::from_millis(20));
        backend
            .start_build(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
            })
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), backend.cancel_build())
            .await
            .unwrap()
            .unwrap();
        fs::remove_file(script).unwrap();
    }

    #[tokio::test]
    async fn process_backend_reports_exit_code() {
        let script = fixture_script("failed-bitbake");
        fs::write(
            &script,
            "#!/bin/sh\nprintf 'ERROR: failed build\\n' >&2\nexit 7\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();
        let mut backend = shell_backend(script.clone());
        backend
            .start_build(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: None,
            })
            .await
            .unwrap();
        loop {
            if let BackendEvent::BuildCompleted { success, exit_code } =
                backend.next_event().await.unwrap()
            {
                assert!(!success);
                assert_eq!(exit_code, Some(7));
                break;
            }
        }
        fs::remove_file(script).unwrap();
    }

    #[tokio::test]
    async fn bridge_backend_negotiates_before_workspace_inspection() {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bridge/yoctui_bridge.py");
        let mut backend = BridgeBackend::spawn("python3", script, std::env::temp_dir())
            .await
            .unwrap();
        let workspace = backend.inspect_workspace().await.unwrap();
        assert_eq!(workspace.build_dir, Some(std::env::temp_dir()));
        backend.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn bridge_backend_waits_for_shutdown_acknowledgement() {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bridge/yoctui_bridge.py");
        let mut backend = BridgeBackend::spawn("python3", script, std::env::temp_dir())
            .await
            .unwrap();
        backend.shutdown().await.unwrap();
        assert!(backend.child.try_wait().unwrap().is_some());
    }

    #[tokio::test]
    async fn bridge_backend_decodes_typed_workspace_queries() {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bridge/yoctui_bridge.py");
        let mut backend = BridgeBackend::spawn("python3", script, std::env::temp_dir())
            .await
            .unwrap();
        assert!(backend.list_recipes(None).await.unwrap().is_empty());
        let _layers = backend.list_layers().await.unwrap();
        assert!(
            backend
                .get_variable("PATH".into(), None)
                .await
                .unwrap()
                .value
                .is_some()
        );
        backend.shutdown().await.unwrap();
    }
}
