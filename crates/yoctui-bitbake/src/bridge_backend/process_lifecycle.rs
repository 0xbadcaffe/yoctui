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

}
