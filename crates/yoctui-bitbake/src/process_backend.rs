//! Process backend.
use super::*;

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
        id: 0,
        severity,
        message: clean,
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::now(),
        build: None,
        protected: false,
        diagnostic: None,
    }
}
pub struct ProcessBackend {
    pub(crate) build_dir: PathBuf,
    pub(crate) signature_adapter: SignatureAdapter,
    pub(crate) executable: PathBuf,
    pub(crate) arguments: Vec<OsString>,
    pub(crate) environment: Vec<(OsString, OsString)>,
    pub(crate) compatibility: Option<yoctui_model::DaemonCompatibilitySnapshot>,
    pub(crate) child: Option<Child>,
    pub(crate) output: Option<tokio::sync::mpsc::Receiver<LogEntry>>,
    pub(crate) build_started_pending: bool,
    pub(crate) cancellation_timeout: Duration,
    #[cfg(unix)]
    pub(crate) process_group: Option<i32>,
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
            signature_adapter: SignatureAdapter::new(build_dir.clone()),
            build_dir,
            executable,
            arguments,
            environment: Vec::new(),
            compatibility: None,
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
    pub fn with_environment(mut self, environment: BTreeMap<String, String>) -> Self {
        self.environment = environment
            .into_iter()
            .map(|(key, value)| (OsString::from(key), OsString::from(value)))
            .collect();
        self
    }
    pub fn with_compatibility(
        mut self,
        compatibility: yoctui_model::DaemonCompatibilitySnapshot,
    ) -> Result<Self, BackendError> {
        let compatibility = compatibility
            .normalize()
            .map_err(|error| BackendError::Bridge(error.to_string()))?;
        self.signature_adapter = self
            .signature_adapter
            .with_compatibility(compatibility.clone())?;
        self.compatibility = Some(compatibility);
        Ok(self)
    }
    pub(crate) fn command_planner(&self) -> Result<BitBakeCommandPlanner<'_>, BackendError> {
        let compatibility = self.compatibility.as_ref().ok_or_else(|| {
            BackendError::Bridge(
                "BitBake command is unavailable until the daemon supplies an authoritative capability snapshot"
                    .into(),
            )
        })?;
        BitBakeCommandPlanner::new(
            compatibility,
            compatibility.snapshot.generation,
            &self.build_dir,
        )
        .map_err(|error| BackendError::Bridge(error.to_string()))
    }
    pub(crate) async fn collect(&mut self) -> Result<(bool, Option<i32>), BackendError> {
        let child = self.child.as_mut().ok_or(BackendError::NotRunning)?;
        let status = child.wait().await?;
        Ok((status.success(), status.code()))
    }

    pub(crate) async fn generate_dependency_graph(
        &self,
        recipe: String,
    ) -> Result<DependencyGraphResponse, BackendError> {
        if self.child.is_some() {
            return Err(BackendError::Bridge(
                "dependency graph generation is unavailable during an active build".into(),
            ));
        }
        BuildRequest {
            targets: vec![recipe.clone()],
            task: None,
            force: false,
        }
        .validate()
        .map_err(|error| BackendError::Bridge(error.to_string()))?;

        let graph_path = self.build_dir.join("task-depends.dot");
        match tokio::fs::remove_file(&graph_path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        let authorized = self
            .command_planner()?
            .dependency_graph(&recipe)
            .map_err(|error| BackendError::Bridge(error.to_string()))?;
        if authorized.executable != self.executable {
            return Err(BackendError::Bridge(format!(
                "configured BitBake executable {} does not match capability-authorized executable {}",
                self.executable.display(),
                authorized.executable.display()
            )));
        }
        let mut command = TokioCommand::new(&authorized.executable);
        command.envs(self.environment.iter().map(|(key, value)| (key, value)));
        command
            .args(&self.arguments)
            .args(&authorized.arguments)
            .current_dir(&self.build_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = command.spawn()?;
        let status = match tokio::time::timeout(DEPENDENCY_GRAPH_TIMEOUT, child.wait()).await {
            Ok(status) => status?,
            Err(_) => {
                let _ = child.kill().await;
                return Err(BackendError::Bridge(
                    "BitBake dependency graph generation timed out after 120 seconds".into(),
                ));
            }
        };
        if !status.success() {
            return Err(BackendError::Bridge(format!(
                "BitBake dependency graph generation exited with {}",
                status
                    .code()
                    .map_or_else(|| "no exit code".into(), |code| code.to_string())
            )));
        }

        let metadata = tokio::fs::symlink_metadata(&graph_path).await?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(BackendError::Bridge(
                "BitBake dependency graph output is not a regular file".into(),
            ));
        }
        if metadata.len() > MAX_DEPENDENCY_GRAPH_FILE_BYTES {
            return Err(BackendError::Bridge(format!(
                "BitBake dependency graph exceeds the {} byte limit",
                MAX_DEPENDENCY_GRAPH_FILE_BYTES
            )));
        }
        let canonical_build_dir = tokio::fs::canonicalize(&self.build_dir).await?;
        let canonical_graph = tokio::fs::canonicalize(&graph_path).await?;
        if canonical_graph.parent() != Some(canonical_build_dir.as_path()) {
            return Err(BackendError::Bridge(
                "BitBake dependency graph output escaped the build directory".into(),
            ));
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        tokio::fs::File::open(&canonical_graph)
            .await?
            .take(MAX_DEPENDENCY_GRAPH_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .await?;
        if bytes.len() as u64 > MAX_DEPENDENCY_GRAPH_FILE_BYTES {
            return Err(BackendError::Bridge(
                "BitBake dependency graph grew beyond its byte limit while reading".into(),
            ));
        }
        parse_task_dependency_dot(&recipe, &bytes)
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
    async fn get_dependency_graph(
        &mut self,
        recipe: String,
    ) -> Result<DependencyGraphResponse, BackendError> {
        self.generate_dependency_graph(recipe).await
    }
    async fn get_signature_dump(
        &mut self,
        target: SignatureTarget,
    ) -> Result<SignatureDumpResponse, BackendError> {
        if self.child.is_some() {
            return Err(BackendError::Bridge(
                "signature inspection is unavailable during an active process-backend build".into(),
            ));
        }
        self.signature_adapter
            .dump(target)
            .await
            .map_err(Into::into)
    }
    async fn compare_signatures(
        &mut self,
        request: SignatureComparisonRequest,
    ) -> Result<SignatureComparisonResponse, BackendError> {
        if self.child.is_some() {
            return Err(BackendError::Bridge(
                "signature comparison is unavailable during an active process-backend build".into(),
            ));
        }
        self.signature_adapter
            .compare(request)
            .await
            .map_err(Into::into)
    }
    async fn get_recipe_sources(&mut self, _recipe: String) -> Result<Vec<PathBuf>, BackendError> {
        Err(BackendError::Bridge("the process backend cannot inspect authoritative recipe source paths; use the Yoctui bridge".into()))
    }
    async fn get_recipe_metadata(
        &mut self,
        _recipe: String,
    ) -> Result<RecipeMetadata, BackendError> {
        Err(BackendError::Bridge(
            "the process backend cannot inspect authoritative recipe metadata; use the Yoctui bridge"
                .into(),
        ))
    }
    async fn get_layer_relationships(&mut self) -> Result<Vec<LayerRelationship>, BackendError> {
        Err(BackendError::Bridge("the process backend cannot inspect authoritative layer relationships; use the Yoctui bridge".into()))
    }
    async fn start_build(&mut self, request: BuildRequest) -> Result<(), BackendError> {
        request
            .validate()
            .map_err(|e| BackendError::Bridge(e.to_string()))?;
        let authorized = self
            .command_planner()?
            .build(&request)
            .map_err(|error| BackendError::Bridge(error.to_string()))?;
        if authorized.executable != self.executable {
            return Err(BackendError::Bridge(format!(
                "configured BitBake executable {} does not match capability-authorized executable {}",
                self.executable.display(),
                authorized.executable.display()
            )));
        }
        let mut cmd = TokioCommand::new(&authorized.executable);
        cmd.args(&self.arguments);
        cmd.envs(self.environment.iter().map(|(key, value)| (key, value)));
        cmd.args(&authorized.arguments)
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
