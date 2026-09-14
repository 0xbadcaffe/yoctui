//! Bridge backend.
use super::*;

pub struct BridgeBackend {
    pub(crate) child: Child,
    pub(crate) stdin: ChildStdin,
    pub(crate) lines: BufReader<tokio::process::ChildStdout>,
    pub(crate) sequence: u64,
    pub(crate) last_sequence: u64,
    pub(crate) accepted_correlations: VecDeque<String>,
    pub(crate) signature_adapter: SignatureAdapter,
    pub(crate) api_authority: Option<BitBakeApiAuthority>,
    pub(crate) stderr_tail: Arc<Mutex<BridgeStderrTail>>,
    pub(crate) stderr_task: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BridgeProcessPriority {
    #[default]
    Inherited,
    Background,
}

const BACKGROUND_BRIDGE_NICE: i32 = 10;

#[derive(Default)]
pub(crate) struct BridgeStderrTail {
    pub(crate) bytes: VecDeque<u8>,
    pub(crate) truncated: bool,
}

impl BridgeStderrTail {
    pub(crate) fn push(&mut self, bytes: &[u8]) {
        if bytes.len() >= MAX_BRIDGE_STDERR_BYTES {
            self.bytes.clear();
            self.bytes.extend(
                bytes[bytes.len().saturating_sub(MAX_BRIDGE_STDERR_BYTES)..]
                    .iter()
                    .copied(),
            );
            self.truncated = true;
            return;
        }
        let overflow = self
            .bytes
            .len()
            .saturating_add(bytes.len())
            .saturating_sub(MAX_BRIDGE_STDERR_BYTES);
        if overflow > 0 {
            self.bytes.drain(..overflow);
            self.truncated = true;
        }
        self.bytes.extend(bytes.iter().copied());
    }

    pub(crate) fn diagnostic(&self) -> Option<String> {
        if self.bytes.is_empty() {
            return None;
        }
        let bytes = self.bytes.iter().copied().collect::<Vec<_>>();
        let text = String::from_utf8_lossy(&bytes);
        let mut output = String::new();
        if self.truncated {
            output.push_str("[earlier bridge stderr truncated]\n");
        }
        for line in text.lines() {
            let normalized = line.to_ascii_lowercase();
            if [
                "password",
                "passwd",
                "secret",
                "token",
                "credential",
                "api_key",
            ]
            .iter()
            .any(|word| normalized.contains(word))
            {
                output.push_str("[redacted sensitive diagnostic]");
            } else {
                output.extend(line.chars().map(|character| {
                    if character.is_control() && character != '\t' {
                        '�'
                    } else {
                        character
                    }
                }));
            }
            output.push('\n');
        }
        let output = output.trim().to_owned();
        (!output.is_empty()).then_some(output)
    }
}

pub(crate) async fn drain_bridge_stderr<R>(mut stderr: R, tail: Arc<Mutex<BridgeStderrTail>>)
where
    R: AsyncRead + Unpin,
{
    let mut buffer = [0_u8; 4096];
    loop {
        let count = match stderr.read(&mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(count) => count,
        };
        if let Ok(mut tail) = tail.lock() {
            tail.push(&buffer[..count]);
        }
    }
}

pub(crate) const BUNDLED_BRIDGE_SOURCE: &str = include_str!("../bridge/yoctui_bridge.py");

impl BridgeBackend {
    pub async fn spawn_bundled(python: &str, build_dir: PathBuf) -> Result<Self, BackendError> {
        Self::spawn_bundled_with_environment(python, build_dir, BTreeMap::new()).await
    }

    pub async fn spawn_bundled_with_environment(
        python: &str,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
    ) -> Result<Self, BackendError> {
        let mut command = TokioCommand::new(python);
        command.arg("-c").arg(BUNDLED_BRIDGE_SOURCE);
        Self::spawn_command(
            command,
            build_dir,
            environment,
            None,
            BridgeProcessPriority::Inherited,
        )
        .await
    }

    pub async fn spawn_bundled_with_compatibility(
        python: &str,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
        compatibility: yoctui_model::DaemonCompatibilitySnapshot,
        expected_generation: u64,
    ) -> Result<Self, BackendError> {
        let mut command = TokioCommand::new(python);
        command.arg("-c").arg(BUNDLED_BRIDGE_SOURCE);
        let authority = BitBakeApiAuthority::new(compatibility, expected_generation, &build_dir)?;
        Self::spawn_command(
            command,
            build_dir,
            environment,
            Some(authority),
            BridgeProcessPriority::Inherited,
        )
        .await
    }

    pub async fn spawn_bundled_with_compatibility_at_priority(
        python: &str,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
        compatibility: yoctui_model::DaemonCompatibilitySnapshot,
        expected_generation: u64,
        priority: BridgeProcessPriority,
    ) -> Result<Self, BackendError> {
        let mut command = TokioCommand::new(python);
        command.arg("-c").arg(BUNDLED_BRIDGE_SOURCE);
        let authority = BitBakeApiAuthority::new(compatibility, expected_generation, &build_dir)?;
        Self::spawn_command(command, build_dir, environment, Some(authority), priority).await
    }

    pub async fn spawn(
        python: &str,
        script: PathBuf,
        build_dir: PathBuf,
    ) -> Result<Self, BackendError> {
        Self::spawn_with_environment(python, script, build_dir, BTreeMap::new()).await
    }
    pub async fn spawn_with_environment(
        python: &str,
        script: PathBuf,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
    ) -> Result<Self, BackendError> {
        let mut command = TokioCommand::new(python);
        command.arg(script);
        Self::spawn_command(
            command,
            build_dir,
            environment,
            None,
            BridgeProcessPriority::Inherited,
        )
        .await
    }

    pub async fn spawn_with_compatibility(
        python: &str,
        script: PathBuf,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
        compatibility: yoctui_model::DaemonCompatibilitySnapshot,
        expected_generation: u64,
    ) -> Result<Self, BackendError> {
        let mut command = TokioCommand::new(python);
        command.arg(script);
        let authority = BitBakeApiAuthority::new(compatibility, expected_generation, &build_dir)?;
        Self::spawn_command(
            command,
            build_dir,
            environment,
            Some(authority),
            BridgeProcessPriority::Inherited,
        )
        .await
    }

    pub async fn spawn_with_compatibility_at_priority(
        python: &str,
        script: PathBuf,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
        compatibility: yoctui_model::DaemonCompatibilitySnapshot,
        expected_generation: u64,
        priority: BridgeProcessPriority,
    ) -> Result<Self, BackendError> {
        let mut command = TokioCommand::new(python);
        command.arg(script);
        let authority = BitBakeApiAuthority::new(compatibility, expected_generation, &build_dir)?;
        Self::spawn_command(command, build_dir, environment, Some(authority), priority).await
    }

    pub(crate) async fn spawn_command(
        mut command: TokioCommand,
        build_dir: PathBuf,
        environment: BTreeMap<String, String>,
        api_authority: Option<BitBakeApiAuthority>,
        priority: BridgeProcessPriority,
    ) -> Result<Self, BackendError> {
        let mut child = command
            .current_dir(&build_dir)
            .envs(environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        if priority == BridgeProcessPriority::Background
            && let Some(pid) = child.id()
            && let Err(error) = yoctui_utils::lower_process_priority(pid, BACKGROUND_BRIDGE_NICE)
        {
            tracing::warn!(pid, %error, "could not lower startup metadata bridge priority");
        }
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| BackendError::Bridge("bridge stdin unavailable".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BackendError::Bridge("bridge stdout unavailable".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| BackendError::Bridge("bridge stderr unavailable".into()))?;
        let stderr_tail = Arc::new(Mutex::new(BridgeStderrTail::default()));
        let stderr_task = tokio::spawn(drain_bridge_stderr(stderr, Arc::clone(&stderr_tail)));
        let signature_adapter = if let Some(authority) = api_authority.as_ref() {
            SignatureAdapter::new(build_dir.clone())
                .with_compatibility(authority.compatibility_snapshot().clone())?
        } else {
            SignatureAdapter::new(build_dir.clone())
        };
        let mut backend = Self {
            child,
            stdin,
            lines: BufReader::new(stdout),
            sequence: 0,
            last_sequence: 0,
            accepted_correlations: VecDeque::new(),
            signature_adapter,
            api_authority,
            stderr_tail,
            stderr_task: Some(stderr_task),
        };
        if let Err(error) = backend.handshake().await {
            backend.stop_failed_startup().await;
            return Err(backend.with_stderr_context(error));
        }
        Ok(backend)
    }

    pub(crate) fn stderr_diagnostic(&self) -> Option<String> {
        self.stderr_tail.lock().ok()?.diagnostic()
    }

    pub(crate) fn with_stderr_context(&self, error: BackendError) -> BackendError {
        let Some(diagnostic) = self.stderr_diagnostic() else {
            return error;
        };
        BackendError::Bridge(format!("{error}; bridge stderr: {diagnostic}"))
    }

    pub(crate) fn disconnected(&self, context: &str) -> BackendError {
        self.with_stderr_context(BackendError::Bridge(context.into()))
    }

    pub(crate) async fn finish_stderr_capture(&mut self) {
        let Some(mut task) = self.stderr_task.take() else {
            return;
        };
        if tokio::time::timeout(Duration::from_secs(1), &mut task)
            .await
            .is_err()
        {
            task.abort();
        }
    }

    pub(crate) async fn stop_failed_startup(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.start_kill();
            let _ = self.child.wait().await;
        }
        self.finish_stderr_capture().await;
    }
    pub(crate) async fn command(&mut self, message: Command) -> Result<(), BackendError> {
        self.sequence += 1;
        let correlation = self.sequence.to_string();
        self.accepted_correlations.push_back(correlation.clone());
        while self.accepted_correlations.len() > 32 {
            self.accepted_correlations.pop_front();
        }
        let bytes = encode_line(&Envelope {
            protocol_version: VERSION,
            sequence: self.sequence,
            correlation_id: Some(correlation),
            message,
        })?;
        self.stdin.write_all(&bytes).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    pub(crate) fn validate_correlation(
        &self,
        envelope: &Envelope<Event>,
    ) -> Result<(), BackendError> {
        let Some(correlation) = envelope.correlation_id.as_ref() else {
            return Err(BackendError::Bridge(
                "bridge response omitted its command correlation".into(),
            ));
        };
        if !self.accepted_correlations.contains(correlation) {
            return Err(BackendError::Bridge(format!(
                "bridge response used unknown correlation {correlation}"
            )));
        }
        Ok(())
    }

    pub(crate) async fn next_line(&mut self) -> Result<Option<Vec<u8>>, BackendError> {
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

    pub(crate) async fn handshake(&mut self) -> Result<(), BackendError> {
        let compatibility = self
            .api_authority
            .as_ref()
            .map(|authority| Box::new(authority.bridge_handshake()));
        self.command(Command::Hello { compatibility }).await?;
        let Some(line) = self.next_line().await? else {
            return Err(self.disconnected("bridge disconnected during protocol handshake"));
        };
        let envelope: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
        self.validate_correlation(&envelope)?;
        self.last_sequence = envelope.sequence;
        match envelope.message {
            Event::HelloAck {
                compatibility_generation,
                capabilities,
                ..
            } => {
                if let Some(authority) = self.api_authority.as_mut() {
                    authority.accept_negotiation(compatibility_generation, &capabilities)?;
                }
                Ok(())
            }
            Event::ProtocolError { code, message } | Event::CommandFailed { code, message } => Err(
                BackendError::Bridge(format!("handshake rejected: {code}: {message}")),
            ),
            _ => Err(BackendError::Bridge(
                "bridge sent an unexpected handshake event".into(),
            )),
        }
    }

    pub(crate) fn require_api(&self, operation: BitBakeApiOperation) -> Result<(), BackendError> {
        self.api_authority
            .as_ref()
            .ok_or_else(|| {
                BackendError::Bridge(
                    "BitBake API operation requires a daemon capability snapshot".into(),
                )
            })?
            .require(operation)
            .map_err(Into::into)
    }

    pub fn supports_api(&self, operation: BitBakeApiOperation) -> bool {
        self.api_authority
            .as_ref()
            .is_some_and(|authority| authority.require(operation).is_ok())
    }

    /// Interrupt an owned metadata query so Python can release its Tinfoil
    /// connection even while waiting for a synchronous recipe parse. Never
    /// use this to cancel a build: builds require the typed cancellation path.
    #[cfg(unix)]
    pub async fn interrupt_metadata(&mut self) {
        if self.child.try_wait().ok().flatten().is_none()
            && let Some(pid) = self.child.id()
        {
            // SAFETY: this PID belongs to our unreaped bridge child, not the
            // shared BitBake server or another user's process group.
            unsafe {
                libc::kill(pid as i32, libc::SIGINT);
            }
            if tokio::time::timeout(Duration::from_secs(5), self.child.wait())
                .await
                .is_err()
            {
                let _ = self.child.start_kill();
                let _ = self.child.wait().await;
            }
        }
        self.finish_stderr_capture().await;
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
        self.validate_correlation(&envelope)?;
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
        self.finish_stderr_capture().await;
        Ok(())
    }

    /// Terminate the connected BitBake process server through Tinfoil's
    /// supported process-server connection, then wait for the bridge to exit.
    pub async fn terminate_server(&mut self) -> Result<(), BackendError> {
        self.require_api(BitBakeApiOperation::ServerSocket)?;
        self.command(Command::TerminateServer).await?;
        let Some(line) = self.next_line().await? else {
            return Err(BackendError::Bridge(
                "bridge disconnected before acknowledging server termination".into(),
            ));
        };
        let envelope: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
        self.validate_correlation(&envelope)?;
        self.last_sequence = envelope.sequence;
        match envelope.message {
            Event::ServerTerminated => {}
            Event::CommandFailed { code, message } | Event::ProtocolError { code, message } => {
                return Err(BackendError::Bridge(format!(
                    "server termination rejected: {code}: {message}"
                )));
            }
            _ => {
                return Err(BackendError::Bridge(
                    "bridge sent an unexpected server termination event".into(),
                ));
            }
        }
        tokio::time::timeout(Duration::from_secs(2), self.child.wait())
            .await
            .map_err(|_| {
                BackendError::Bridge(
                    "bridge did not exit after server termination acknowledgement".into(),
                )
            })??;
        self.finish_stderr_capture().await;
        Ok(())
    }
    pub(crate) fn event(event: Event) -> Result<BackendEvent, BackendError> {
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
                        preferred_version: recipe.preferred_version,
                        file: recipe.file.map(PathBuf::from),
                        append_count: recipe.append_count,
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
                             preferred_version,
                             file,
                             append_count,
                         }| Recipe {
                            name,
                            version,
                            layer,
                            preferred_version,
                            file: file.map(PathBuf::from),
                            append_count,
                        },
                    )
                    .collect(),
            ),
            Event::RecipesChunk { .. } => {
                return Err(BackendError::Bridge(
                    "recipe chunk outside an active inventory request".into(),
                ));
            }
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
                recipe,
                value,
                provenance,
                unexpanded_value,
                operations,
                active_overrides,
            } => BackendEvent::Variable {
                name,
                recipe,
                value,
                provenance,
                unexpanded_value,
                operations: operations
                    .into_iter()
                    .map(|operation| VariableOperation {
                        operation: operation.operation,
                        file: operation.file.map(PathBuf::from),
                        line: operation.line,
                        value: operation.value,
                    })
                    .collect(),
                active_overrides,
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
            Event::DependencyGraph { data } => {
                let DependencyGraphData {
                    root,
                    nodes,
                    edges,
                    mut limitations,
                } = data;
                let root = dependency_node_id(root)?;
                let mut dropped_paths = 0;
                let nodes = nodes
                    .into_iter()
                    .map(|DependencyNodeData { id, provider, log }| {
                        let provider = provider.map(PathBuf::from).and_then(|path| {
                            if path.is_absolute() {
                                Some(path)
                            } else {
                                dropped_paths += 1;
                                None
                            }
                        });
                        let log = log.map(PathBuf::from).and_then(|path| {
                            if path.is_absolute() {
                                Some(path)
                            } else {
                                dropped_paths += 1;
                                None
                            }
                        });
                        Ok(DependencyNode {
                            id: dependency_node_id(id)?,
                            provider,
                            log,
                        })
                    })
                    .collect::<Result<Vec<_>, BackendError>>()?;
                let edges = edges
                    .into_iter()
                    .map(|DependencyEdgeData { from, to, kind }| {
                        Ok(DependencyEdge {
                            from: dependency_node_id(from)?,
                            to: dependency_node_id(to)?,
                            kind: match kind {
                                DependencyEdgeKindData::Build => DependencyEdgeKind::Build,
                                DependencyEdgeKindData::Runtime => DependencyEdgeKind::Runtime,
                                DependencyEdgeKindData::Task => DependencyEdgeKind::Task,
                            },
                        })
                    })
                    .collect::<Result<Vec<_>, BackendError>>()?;
                let (graph, report) = DependencyGraph::normalize(
                    root,
                    nodes,
                    edges,
                    MAX_DEPENDENCY_NODES,
                    MAX_DEPENDENCY_EDGES,
                );
                if report.is_partial() {
                    limitations.push(format!(
                        "Rust adapter bounds dropped {} nodes and {} edges",
                        report.truncated_nodes, report.truncated_edges
                    ));
                }
                if dropped_paths > 0 {
                    limitations.push(format!(
                        "Rust adapter dropped {dropped_paths} non-absolute provider or log paths"
                    ));
                }
                BackendEvent::DependencyGraph { graph, limitations }
            }
            Event::RecipeSources { recipe, paths } => BackendEvent::RecipeSources {
                recipe,
                paths: paths.into_iter().map(PathBuf::from).collect(),
            },
            Event::RecipeMetadata { data } => BackendEvent::RecipeMetadata(RecipeMetadata {
                recipe: data.recipe,
                workspace_status: data.workspace_status.map(|status| match status {
                    RecipeWorkspaceStatusData::Clean => RecipeWorkspaceStatus::Clean,
                    RecipeWorkspaceStatusData::Modified => RecipeWorkspaceStatus::Modified,
                }),
                build_status: data.build_status.map(|status| match status {
                    RecipeBuildStatusData::Idle => RecipeBuildStatus::Idle,
                    RecipeBuildStatusData::Queued => RecipeBuildStatus::Queued,
                    RecipeBuildStatusData::Running => RecipeBuildStatus::Running,
                    RecipeBuildStatusData::Succeeded => RecipeBuildStatus::Succeeded,
                    RecipeBuildStatusData::Failed => RecipeBuildStatus::Failed,
                    RecipeBuildStatusData::Cancelled => RecipeBuildStatus::Cancelled,
                }),
                tasks: data.tasks,
                sources: data
                    .sources
                    .map(|paths| paths.into_iter().map(PathBuf::from).collect()),
                patches: data.patches,
                packages: data.packages,
                history: data.history,
            }),
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
            Event::TaskStats { stats } => BackendEvent::TaskStats(TaskStats {
                completed: stats.completed,
                total: stats.total,
                active: stats.active,
                failed: stats.failed,
            }),
            Event::ParseProgress { current, total } => {
                BackendEvent::ParseProgress { current, total }
            }
            Event::TaskQueued {
                recipe,
                task,
                worker,
                stats,
            } => BackendEvent::TaskQueued {
                recipe,
                task,
                worker,
                stats: task_stats(stats),
            },
            Event::TaskStarted {
                recipe,
                task,
                pid,
                worker,
                log_path,
                stats,
            } => BackendEvent::TaskStarted {
                recipe,
                task,
                pid,
                worker,
                log_path: log_path.map(PathBuf::from),
                stats: task_stats(stats),
            },
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
                    id: 0,
                    severity,
                    message,
                    recipe,
                    task,
                    path: path.map(PathBuf::from),
                    timestamp: SystemTime::now(),
                    build: None,
                    protected: false,
                    diagnostic: None,
                })
            }
            Event::Warning { message } => BackendEvent::Log(LogEntry {
                id: 0,
                severity: Severity::Warning,
                message,
                recipe: None,
                task: None,
                path: None,
                timestamp: SystemTime::now(),
                build: None,
                protected: true,
                diagnostic: None,
            }),
            Event::Error { message } => BackendEvent::Log(LogEntry {
                id: 0,
                severity: Severity::Error,
                message,
                recipe: None,
                task: None,
                path: None,
                timestamp: SystemTime::now(),
                build: None,
                protected: true,
                diagnostic: None,
            }),
            Event::BuildCompleted { success, exit_code } => {
                BackendEvent::BuildCompleted { success, exit_code }
            }
            Event::CommandFailed { code, message } | Event::ProtocolError { code, message } => {
                BackendEvent::CommandFailed { code, message }
            }
            Event::BridgeShutdown | Event::ServerTerminated => BackendEvent::Disconnected,
            Event::HelloAck { .. } | Event::Unknown => BackendEvent::Ignored,
        })
    }
}

pub(crate) fn task_stats(data: Option<TaskStatsData>) -> Option<TaskStats> {
    data.map(|stats| TaskStats {
        completed: stats.completed,
        total: stats.total,
        active: stats.active,
        failed: stats.failed,
    })
}

pub(crate) fn dependency_node_id(
    data: DependencyNodeIdData,
) -> Result<DependencyNodeId, BackendError> {
    if data.recipe.is_empty()
        || data.recipe.len() > 512
        || data.recipe.chars().any(char::is_whitespace)
        || data.recipe.chars().any(char::is_control)
        || data.task.as_ref().is_some_and(|task| {
            task.is_empty()
                || task.len() > 512
                || task.chars().any(char::is_whitespace)
                || task.chars().any(char::is_control)
        })
    {
        return Err(BackendError::Bridge(
            "protocol dependency graph contains an invalid node identity".into(),
        ));
    }
    Ok(match data.task {
        Some(task) => DependencyNodeId::task(data.recipe, task),
        None => DependencyNodeId::recipe(data.recipe),
    })
}

pub(crate) fn legacy_dependency_graph(
    recipe: String,
    build: Vec<String>,
    runtime: Vec<String>,
) -> DependencyGraphResponse {
    let root = DependencyNodeId::recipe(recipe);
    let edges = build
        .into_iter()
        .map(|dependency| DependencyEdge {
            from: root.clone(),
            to: DependencyNodeId::recipe(dependency),
            kind: DependencyEdgeKind::Build,
        })
        .chain(runtime.into_iter().map(|dependency| DependencyEdge {
            from: root.clone(),
            to: DependencyNodeId::recipe(dependency),
            kind: DependencyEdgeKind::Runtime,
        }))
        .collect();
    let (graph, _) = DependencyGraph::normalize(
        root,
        Vec::new(),
        edges,
        MAX_DEPENDENCY_NODES,
        MAX_DEPENDENCY_EDGES,
    );
    DependencyGraphResponse {
        graph,
        limitations: vec![
            "Legacy bridge supplied direct recipe edges only; task dependencies are unavailable."
                .into(),
        ],
    }
}

pub(crate) fn dot_quoted_id(line: &str) -> Result<(String, &str), BackendError> {
    let Some(content) = line.strip_prefix('"') else {
        return Err(BackendError::Bridge(
            "malformed dependency graph identifier".into(),
        ));
    };
    let mut value = String::new();
    let mut escaped = false;
    for (offset, character) in content.char_indices() {
        if escaped {
            match character {
                '"' | '\\' => value.push(character),
                _ => {
                    return Err(BackendError::Bridge(
                        "unsupported escape in dependency graph identifier".into(),
                    ));
                }
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Ok((value, &content[offset + character.len_utf8()..]));
        } else {
            value.push(character);
        }
    }
    Err(BackendError::Bridge(
        "unterminated dependency graph identifier".into(),
    ))
}

pub(crate) fn dependency_task_identity(value: String) -> Result<DependencyNodeId, BackendError> {
    let Some((recipe, task)) = value.rsplit_once('.') else {
        return Err(BackendError::Bridge(
            "dependency graph task identity has no task separator".into(),
        ));
    };
    if recipe.is_empty()
        || task.is_empty()
        || recipe.len() > 512
        || task.len() > 512
        || recipe.chars().any(char::is_whitespace)
        || task.chars().any(char::is_whitespace)
        || recipe.chars().any(char::is_control)
        || task.chars().any(char::is_control)
    {
        return Err(BackendError::Bridge(
            "dependency graph contains an invalid task identity".into(),
        ));
    }
    Ok(DependencyNodeId::task(recipe, task))
}

pub(crate) fn parse_task_dependency_dot(
    recipe: &str,
    bytes: &[u8],
) -> Result<DependencyGraphResponse, BackendError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| BackendError::Bridge("dependency graph is not valid UTF-8".into()))?;
    let root = DependencyNodeId::recipe(recipe);
    let mut nodes = vec![DependencyNode::identity(root.clone())];
    let mut edges = Vec::new();
    let mut opened = false;
    let mut closed = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !opened {
            if line != "digraph depends {" {
                return Err(BackendError::Bridge(
                    "dependency graph has an invalid header".into(),
                ));
            }
            opened = true;
            continue;
        }
        if line == "}" {
            closed = true;
            continue;
        }
        if closed {
            return Err(BackendError::Bridge(
                "dependency graph contains records after its closing brace".into(),
            ));
        }

        let (source, remainder) = dot_quoted_id(line)?;
        let remainder = remainder.trim_start();
        if remainder.starts_with('[') {
            if !remainder.trim_end_matches(';').ends_with(']') {
                return Err(BackendError::Bridge(
                    "dependency graph node contains malformed attributes".into(),
                ));
            }
            nodes.push(DependencyNode::identity(dependency_task_identity(source)?));
            continue;
        }
        let Some(remainder) = remainder.strip_prefix("->") else {
            return Err(BackendError::Bridge(
                "dependency graph contains an unsupported record".into(),
            ));
        };
        let (destination, trailing) = dot_quoted_id(remainder.trim_start())?;
        if !trailing.trim().trim_end_matches(';').is_empty() {
            return Err(BackendError::Bridge(
                "dependency graph edge contains unsupported attributes".into(),
            ));
        }
        let from = dependency_task_identity(source)?;
        let to = dependency_task_identity(destination)?;
        nodes.push(DependencyNode::identity(from.clone()));
        nodes.push(DependencyNode::identity(to.clone()));
        for task in [&from, &to] {
            let recipe_node = DependencyNodeId::recipe(task.recipe_name());
            nodes.push(DependencyNode::identity(recipe_node.clone()));
            // `bitbake -g` emits task-to-task edges but the workspace root is
            // a recipe node. Preserve an explicit recipe-to-task bridge so
            // task dependencies are reachable instead of becoming orphans.
            edges.push(DependencyEdge {
                from: recipe_node,
                to: task.clone(),
                kind: DependencyEdgeKind::Task,
            });
        }
        edges.push(DependencyEdge {
            from: from.clone(),
            to: to.clone(),
            kind: DependencyEdgeKind::Task,
        });
        if from.recipe_name() != to.recipe_name() {
            edges.push(DependencyEdge {
                from: DependencyNodeId::recipe(from.recipe_name()),
                to: DependencyNodeId::recipe(to.recipe_name()),
                kind: DependencyEdgeKind::Build,
            });
        }
    }
    if !opened || !closed {
        return Err(BackendError::Bridge(
            "dependency graph is incomplete".into(),
        ));
    }
    let (graph, report) = DependencyGraph::normalize(
        root,
        nodes,
        edges,
        MAX_DEPENDENCY_NODES,
        MAX_DEPENDENCY_EDGES,
    );
    let mut limitations = vec![
        "The process backend task graph does not report runtime dependency edges.".into(),
        "The process backend task graph does not report provider or task-log paths.".into(),
    ];
    if report.is_partial() {
        limitations.push(format!(
            "Dependency graph bounds dropped {} nodes and {} edges.",
            report.truncated_nodes, report.truncated_edges
        ));
    }
    Ok(DependencyGraphResponse { graph, limitations })
}
#[async_trait]
impl BitBakeBackend for BridgeBackend {
    async fn inspect_workspace(&mut self) -> Result<Workspace, BackendError> {
        self.require_api(BitBakeApiOperation::Workspace)?;
        self.command(Command::InspectWorkspace).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Workspace(workspace) => return Ok(workspace),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(self.disconnected("bridge disconnected during inspection"));
                }
                _ => {}
            }
        }
    }
    async fn list_recipes(&mut self, filter: Option<String>) -> Result<Vec<Recipe>, BackendError> {
        self.require_api(BitBakeApiOperation::Recipes)?;
        self.command(Command::ListRecipes {
            filter,
            chunked: true,
        })
        .await?;
        let correlation = self.sequence.to_string();
        let mut inventory = recipe_inventory::RecipeInventory::default();
        loop {
            let Some(line) = self.next_line().await? else {
                return Err(
                    self.disconnected("bridge disconnected before recipe inventory completion")
                );
            };
            let envelope: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
            if envelope.correlation_id.as_deref() != Some(correlation.as_str()) {
                return Err(BackendError::Bridge(
                    "recipe inventory response has wrong request correlation".into(),
                ));
            }
            self.last_sequence = envelope.sequence;
            let completed = match envelope.message {
                Event::RecipesChunk {
                    offset,
                    total,
                    complete,
                    recipes,
                } => {
                    if line.len() > yoctui_protocol::MAX_RECIPE_CHUNK_BYTES {
                        return Err(BackendError::Bridge("recipe chunk exceeds 512 KiB".into()));
                    }
                    inventory.push(offset, total, complete, recipes)?
                }
                Event::Recipes { recipes } => inventory.push(0, recipes.len(), true, recipes)?,
                Event::CommandFailed { code, message } | Event::ProtocolError { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                _ => false,
            };
            if completed {
                let BackendEvent::Recipes(recipes) = Self::event(Event::Recipes {
                    recipes: inventory.finish()?,
                })?
                else {
                    unreachable!("recipe conversion")
                };
                return Ok(recipes);
            }
        }
    }
    async fn list_layers(&mut self) -> Result<Vec<Layer>, BackendError> {
        self.require_api(BitBakeApiOperation::Layers)?;
        self.command(Command::ListLayers).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Layers(layers) => return Ok(layers),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(self.disconnected("bridge disconnected while listing layers"));
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
        self.require_api(BitBakeApiOperation::Variable)?;
        let requested_recipe = recipe.clone();
        self.command(Command::GetVariable {
            name: name.clone(),
            recipe,
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::Variable {
                    name: returned,
                    recipe,
                    value,
                    provenance,
                    unexpanded_value,
                    operations,
                    active_overrides,
                } if returned == name && recipe == requested_recipe => {
                    return Ok(VariableValue {
                        recipe,
                        value,
                        provenance,
                        unexpanded_value,
                        operations,
                        active_overrides,
                    });
                }
                BackendEvent::Variable { .. } => continue,
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(self.disconnected("bridge disconnected while reading a variable"));
                }
                _ => {}
            }
        }
    }
    async fn get_dependencies(
        &mut self,
        recipe: String,
    ) -> Result<RecipeDependencies, BackendError> {
        self.require_api(BitBakeApiOperation::Dependencies)?;
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
                    return Err(
                        self.disconnected("bridge disconnected while reading recipe dependencies")
                    );
                }
                _ => continue,
            }
        }
    }
    async fn get_dependency_graph(
        &mut self,
        recipe: String,
    ) -> Result<DependencyGraphResponse, BackendError> {
        self.require_api(BitBakeApiOperation::DependencyGraph)?;
        self.command(Command::GetDependencyGraph {
            recipe: recipe.clone(),
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::DependencyGraph { graph, limitations }
                    if graph.root.recipe_name() == recipe =>
                {
                    return Ok(DependencyGraphResponse { graph, limitations });
                }
                BackendEvent::DependencyGraph { graph, .. } => {
                    return Err(BackendError::Bridge(format!(
                        "bridge returned dependency graph root {} for requested recipe {recipe}",
                        graph.root.recipe_name()
                    )));
                }
                BackendEvent::Dependencies {
                    recipe: returned,
                    build,
                    runtime,
                } if returned == recipe => {
                    return Ok(legacy_dependency_graph(recipe, build, runtime));
                }
                BackendEvent::CommandFailed { code, .. }
                    if code == "invalid_request" || code == "unsupported_command" =>
                {
                    break;
                }
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(
                        self.disconnected("bridge disconnected while reading the dependency graph")
                    );
                }
                _ => continue,
            }
        }
        let dependencies = self.get_dependencies(recipe.clone()).await?;
        Ok(legacy_dependency_graph(
            recipe,
            dependencies.build,
            dependencies.runtime,
        ))
    }
    async fn get_signature_dump(
        &mut self,
        target: SignatureTarget,
    ) -> Result<SignatureDumpResponse, BackendError> {
        self.signature_adapter
            .dump(target)
            .await
            .map_err(Into::into)
    }
    async fn compare_signatures(
        &mut self,
        request: SignatureComparisonRequest,
    ) -> Result<SignatureComparisonResponse, BackendError> {
        self.signature_adapter
            .compare(request)
            .await
            .map_err(Into::into)
    }
    async fn get_recipe_sources(&mut self, recipe: String) -> Result<Vec<PathBuf>, BackendError> {
        self.require_api(BitBakeApiOperation::RecipeSources)?;
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
                    return Err(
                        self.disconnected("bridge disconnected while reading recipe source paths")
                    );
                }
                _ => continue,
            }
        }
    }
    async fn get_recipe_metadata(
        &mut self,
        recipe: String,
    ) -> Result<RecipeMetadata, BackendError> {
        self.require_api(BitBakeApiOperation::RecipeMetadata)?;
        self.command(Command::GetRecipeMetadata {
            recipe: recipe.clone(),
        })
        .await?;
        loop {
            match self.next_event().await? {
                BackendEvent::RecipeMetadata(metadata) if metadata.recipe == recipe => {
                    return Ok(metadata);
                }
                BackendEvent::RecipeMetadata(_) => continue,
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(
                        self.disconnected("bridge disconnected while reading recipe metadata")
                    );
                }
                _ => continue,
            }
        }
    }
    async fn get_layer_relationships(&mut self) -> Result<Vec<LayerRelationship>, BackendError> {
        self.require_api(BitBakeApiOperation::LayerRelationships)?;
        self.command(Command::GetLayerRelationships).await?;
        loop {
            match self.next_event().await? {
                BackendEvent::LayerRelationships(layers) => return Ok(layers),
                BackendEvent::CommandFailed { code, message } => {
                    return Err(BackendError::Bridge(format!("{code}: {message}")));
                }
                BackendEvent::Disconnected => {
                    return Err(
                        self.disconnected("bridge disconnected while reading layer relationships")
                    );
                }
                _ => continue,
            }
        }
    }
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError> {
        self.require_api(BitBakeApiOperation::Build)?;
        if request.force {
            self.require_api(BitBakeApiOperation::ForceTask)?;
        }
        self.command(Command::StartBuild {
            targets: request.targets,
            task: request.task,
            force: request.force,
        })
        .await
    }
    async fn cancel_build(&mut self) -> Result<(), BackendError> {
        self.require_api(BitBakeApiOperation::Cancel)?;
        self.command(Command::CancelBuild).await
    }
    async fn next_event(&mut self) -> Result<BackendEvent, BackendError> {
        let Some(line) = self.next_line().await? else {
            return Ok(BackendEvent::Disconnected);
        };
        let e: Envelope<Event> = decode_line(&line, Some(self.last_sequence))?;
        self.validate_correlation(&e)?;
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
        if let Some(task) = self.stderr_task.take() {
            task.abort();
        }
    }
}
