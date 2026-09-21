pub struct DevtoolJobRunner {
    pub(crate) build_dir: PathBuf,
    pub(crate) child: Option<Child>,
    pub(crate) output: Option<tokio::sync::mpsc::Receiver<DevtoolPipeEvent>>,
    pub(crate) streams_drained: bool,
    pub(crate) started_pending: bool,
    pub(crate) terminal_pending: Option<DevtoolRunnerEvent>,
    pub(crate) cancellation_timeout: Duration,
    pub(crate) cancellation_requested: bool,
    #[cfg(unix)]
    pub(crate) process_group: Option<i32>,
}
impl DevtoolJobRunner {
    pub fn new(build_dir: PathBuf) -> Self {
        Self {
            build_dir,
            child: None,
            output: None,
            streams_drained: true,
            started_pending: false,
            terminal_pending: None,
            cancellation_timeout: Duration::from_secs(5),
            cancellation_requested: false,
            #[cfg(unix)]
            process_group: None,
        }
    }

    pub fn with_cancellation_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_timeout = timeout;
        self
    }

    pub fn is_active(&self) -> bool {
        self.child.is_some()
    }

    pub async fn start(&mut self, command: DevtoolCommandSpec) -> Result<(), DevtoolRunnerError> {
        if self.child.is_some()
            || self.started_pending
            || self.terminal_pending.is_some()
            || self.output.is_some()
        {
            return Err(DevtoolRunnerError::Busy);
        }
        if !self.build_dir.is_dir() {
            return Err(DevtoolRunnerError::Spawn(format!(
                "build directory does not exist: {}",
                self.build_dir.display()
            )));
        }
        if command.capability_generation == 0 || command.build_directory != self.build_dir {
            return Err(DevtoolRunnerError::AuthorizationMismatch);
        }
        let mut process = TokioCommand::new(&command.executable);
        process
            .args(&command.arguments)
            .current_dir(&self.build_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        process.process_group(0);
        let mut child = process.spawn().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                DevtoolRunnerError::MissingExecutable(command.executable.clone())
            } else {
                DevtoolRunnerError::Spawn(error.to_string())
            }
        })?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            return Err(DevtoolRunnerError::StreamUnavailable(
                DevtoolOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill().await;
            return Err(DevtoolRunnerError::StreamUnavailable(
                DevtoolOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(1024);
        tokio::spawn(read_devtool_output(
            stdout,
            DevtoolOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_devtool_output(
            stderr,
            DevtoolOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.child = Some(child);
        self.output = Some(receiver);
        self.streams_drained = false;
        self.started_pending = true;
        self.cancellation_requested = false;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<DevtoolRunnerEvent, DevtoolRunnerError> {
        if self.started_pending {
            self.started_pending = false;
            return Ok(DevtoolRunnerEvent::Started);
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            if let Some(child) = self.child.as_mut() {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            self.child = None;
            self.streams_drained = true;
            #[cfg(unix)]
            {
                self.process_group = None;
            }
            return Ok(DevtoolRunnerEvent::Lost {
                message: "Devtool output event channel was lost".into(),
            });
        }
        if let Some(receiver) = self.output.as_mut() {
            match receiver.recv().await {
                Some(DevtoolPipeEvent::Output {
                    stream,
                    line,
                    truncated,
                }) => {
                    return Ok(DevtoolRunnerEvent::Output {
                        stream,
                        line,
                        truncated,
                    });
                }
                Some(DevtoolPipeEvent::Failed { stream, message }) => {
                    if let Some(child) = self.child.as_mut() {
                        let _ = child.kill().await;
                        let _ = child.wait().await;
                    }
                    self.child = None;
                    self.output = None;
                    self.streams_drained = true;
                    #[cfg(unix)]
                    {
                        self.process_group = None;
                    }
                    return Ok(DevtoolRunnerEvent::Lost {
                        message: format!("{stream:?} stream failed: {message}"),
                    });
                }
                None => {
                    self.output = None;
                    self.streams_drained = true;
                }
            }
        }
        if let Some(event) = self.terminal_pending.take() {
            return Ok(event);
        }
        let Some(child) = self.child.as_mut() else {
            return Err(DevtoolRunnerError::NotRunning);
        };
        let status = match child.wait().await {
            Ok(status) => status,
            Err(error) => {
                self.child = None;
                self.cancellation_requested = false;
                #[cfg(unix)]
                {
                    self.process_group = None;
                }
                return Ok(DevtoolRunnerEvent::Lost {
                    message: format!("Devtool process wait failed: {error}"),
                });
            }
        };
        self.child = None;
        self.cancellation_requested = false;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
        if status.success() {
            Ok(DevtoolRunnerEvent::Completed {
                exit_code: status.code(),
            })
        } else {
            Ok(DevtoolRunnerEvent::Failed {
                exit_code: status.code(),
            })
        }
    }

    pub async fn cancel(&mut self) -> Result<bool, DevtoolRunnerError> {
        if self.cancellation_requested {
            return Ok(false);
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(false);
        };
        self.cancellation_requested = true;
        let mut forced = false;
        #[cfg(unix)]
        let status =
            if let Some(process_group) = self.process_group {
                // SAFETY: the group is the child PID created by `process_group(0)`, so the
                // negative PID targets only the spawned Devtool process group.
                let signal = unsafe { libc::kill(-process_group, libc::SIGTERM) };
                if signal != 0 {
                    child
                        .start_kill()
                        .map_err(|error| DevtoolRunnerError::ProcessControl(error.to_string()))?;
                    forced = true;
                }
                match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                    Ok(result) => result
                        .map_err(|error| DevtoolRunnerError::ProcessControl(error.to_string()))?,
                    Err(_) => {
                        // SAFETY: same child-owned process group as the graceful signal.
                        let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                        forced = true;
                        child.wait().await.map_err(|error| {
                            DevtoolRunnerError::ProcessControl(error.to_string())
                        })?
                    }
                }
            } else {
                forced = true;
                child
                    .kill()
                    .await
                    .map_err(|error| DevtoolRunnerError::ProcessControl(error.to_string()))?;
                child
                    .wait()
                    .await
                    .map_err(|error| DevtoolRunnerError::ProcessControl(error.to_string()))?
            };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| DevtoolRunnerError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| DevtoolRunnerError::ProcessControl(error.to_string()))?
        };
        self.child = None;
        self.cancellation_requested = false;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
        self.terminal_pending = Some(DevtoolRunnerEvent::Cancelled {
            forced,
            exit_code: status.code(),
        });
        Ok(true)
    }
}
impl Drop for DevtoolJobRunner {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(process_group) = self.process_group {
            // SAFETY: this is the child-owned process group created by `start`.
            let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
    }
}
