pub struct MaintenanceSstateJobRunner {
    id: Option<MaintenanceSessionId>,
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<PipeEvent>>,
    streams_drained: bool,
    started_pending: bool,
    terminal_pending: VecDeque<MaintenanceSstateRunnerEvent>,
    cancellation_timeout: Duration,
    operation_timeout: Duration,
    deadline: Option<Instant>,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl Default for MaintenanceSstateJobRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl MaintenanceSstateJobRunner {
    pub fn new() -> Self {
        Self {
            id: None,
            child: None,
            output: None,
            streams_drained: true,
            started_pending: false,
            terminal_pending: VecDeque::new(),
            cancellation_timeout: Duration::from_secs(5),
            operation_timeout: SSTATE_OPERATION_TIMEOUT,
            deadline: None,
            cancellation_requested: false,
            #[cfg(unix)]
            process_group: None,
        }
    }

    pub fn with_cancellation_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_timeout = timeout;
        self
    }

    pub fn with_operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    pub fn is_active(&self) -> bool {
        self.child.is_some()
    }

    pub async fn start(
        &mut self,
        command: MaintenanceSstateCommandSpec,
    ) -> Result<(), MaintenanceSstateAdapterError> {
        if self.child.is_some()
            || self.started_pending
            || !self.terminal_pending.is_empty()
            || self.output.is_some()
        {
            return Err(MaintenanceSstateAdapterError::Busy);
        }
        command.revalidate()?;
        let id = command.id();
        let timeout = command.timeout;
        let stdin_payload = command.stdin_payload.clone();
        let mut process = Command::new(command.executable());
        process
            .args(command.arguments())
            .envs(command.environment())
            .current_dir(command.current_directory())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if stdin_payload.is_some() {
            process.stdin(Stdio::piped());
        } else {
            process.stdin(Stdio::null());
        }
        #[cfg(unix)]
        process.process_group(0);
        let mut child = spawn_process(&mut process)
            .await
            .map_err(|error| MaintenanceSstateAdapterError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        if let Some(payload) = stdin_payload {
            let Some(mut stdin) = child.stdin.take() else {
                let _ = child.kill().await;
                self.clear_process_state();
                return Err(MaintenanceSstateAdapterError::ProcessControl(
                    "cleanup preview stdin is unavailable".into(),
                ));
            };
            if let Err(error) = write_process_stdin(&mut stdin, &payload).await {
                let _ = child.kill().await;
                let _ = child.wait().await;
                self.clear_process_state();
                return Err(MaintenanceSstateAdapterError::ProcessControl(
                    error.to_string(),
                ));
            }
            drop(stdin);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            self.clear_process_state();
            return Err(MaintenanceSstateAdapterError::StreamUnavailable(
                MaintenanceOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill().await;
            self.clear_process_state();
            return Err(MaintenanceSstateAdapterError::StreamUnavailable(
                MaintenanceOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(SSTATE_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_output(
            stdout,
            MaintenanceOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_output(
            stderr,
            MaintenanceOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.id = Some(id);
        self.child = Some(child);
        self.output = Some(receiver);
        self.streams_drained = false;
        self.started_pending = true;
        self.deadline = Some(Instant::now() + timeout.min(self.operation_timeout));
        self.cancellation_requested = false;
        Ok(())
    }

    pub async fn next_event(
        &mut self,
    ) -> Result<MaintenanceSstateRunnerEvent, MaintenanceSstateAdapterError> {
        if let Some(event) = self.terminal_pending.pop_front() {
            return Ok(event);
        }
        let id = self.id.ok_or(MaintenanceSstateAdapterError::NotRunning)?;
        if self.started_pending {
            self.started_pending = false;
            return Ok(MaintenanceSstateRunnerEvent::Started { id });
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            self.kill_and_clear().await;
            return Ok(MaintenanceSstateRunnerEvent::Lost {
                id,
                message: "sstate output event channel was lost".into(),
            });
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return self.timeout_active(id).await;
        }
        if let Some(receiver) = self.output.as_mut() {
            let deadline = self
                .deadline
                .ok_or(MaintenanceSstateAdapterError::NotRunning)?;
            let event = tokio::select! {
                event = receiver.recv() => Some(event),
                _ = tokio::time::sleep_until(deadline) => None,
            };
            match event {
                Some(Some(PipeEvent::Output {
                    stream,
                    line,
                    truncated,
                })) => {
                    return Ok(MaintenanceSstateRunnerEvent::Output {
                        id,
                        stream,
                        line,
                        truncated,
                    });
                }
                Some(Some(PipeEvent::Failed { stream, message })) => {
                    self.kill_and_clear().await;
                    return Ok(MaintenanceSstateRunnerEvent::Lost {
                        id,
                        message: format!("{stream:?} stream failed: {message}"),
                    });
                }
                Some(None) => {
                    self.output = None;
                    self.streams_drained = true;
                }
                None => return self.timeout_active(id).await,
            }
        }
        let deadline = self
            .deadline
            .ok_or(MaintenanceSstateAdapterError::NotRunning)?;
        let status = {
            let child = self
                .child
                .as_mut()
                .ok_or(MaintenanceSstateAdapterError::NotRunning)?;
            match tokio::time::timeout_at(deadline, child.wait()).await {
                Ok(Ok(status)) => status,
                Ok(Err(error)) => {
                    self.kill_and_clear().await;
                    return Ok(MaintenanceSstateRunnerEvent::Lost {
                        id,
                        message: format!("sstate wait failed: {error}"),
                    });
                }
                Err(_) => return self.timeout_active(id).await,
            }
        };
        self.clear_process_state();
        if status.success() {
            Ok(MaintenanceSstateRunnerEvent::Completed {
                id,
                exit_code: status.code(),
            })
        } else {
            Ok(MaintenanceSstateRunnerEvent::Failed {
                id,
                exit_code: status.code(),
            })
        }
    }

    pub async fn cancel(
        &mut self,
        requested_id: MaintenanceSessionId,
    ) -> Result<bool, MaintenanceSstateAdapterError> {
        if self.cancellation_requested || self.child.is_none() || self.id != Some(requested_id) {
            self.terminal_pending
                .push_back(MaintenanceSstateRunnerEvent::CancellationRejected {
                    id: requested_id,
                    message: "no matching cancellable sstate process is active".into(),
                });
            return Ok(false);
        }
        self.cancellation_requested = true;
        self.terminal_pending
            .push_back(MaintenanceSstateRunnerEvent::CancellationRequested { id: requested_id });
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state_preserving_events();
        self.terminal_pending
            .push_back(MaintenanceSstateRunnerEvent::Cancelled {
                id: requested_id,
                forced,
                exit_code: status.and_then(|status| status.code()),
            });
        Ok(true)
    }

    async fn timeout_active(
        &mut self,
        id: MaintenanceSessionId,
    ) -> Result<MaintenanceSstateRunnerEvent, MaintenanceSstateAdapterError> {
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state();
        Ok(MaintenanceSstateRunnerEvent::TimedOut {
            id,
            forced,
            exit_code: status.and_then(|status| status.code()),
        })
    }

    async fn terminate_active(
        &mut self,
    ) -> Result<(Option<std::process::ExitStatus>, bool), MaintenanceSstateAdapterError> {
        let Some(child) = self.child.as_mut() else {
            return Ok((None, false));
        };
        let mut forced = false;
        #[cfg(unix)]
        let status = if let Some(process_group) = self.process_group {
            // SAFETY: the negative PID targets only the process group created for this child.
            if unsafe { libc::kill(-process_group, libc::SIGTERM) } != 0 {
                child.start_kill().map_err(|error| {
                    MaintenanceSstateAdapterError::ProcessControl(error.to_string())
                })?;
                forced = true;
            }
            match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                Ok(result) => Some(result.map_err(|error| {
                    MaintenanceSstateAdapterError::ProcessControl(error.to_string())
                })?),
                Err(_) => {
                    // SAFETY: same child-owned process group as the graceful signal.
                    let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                    forced = true;
                    Some(child.wait().await.map_err(|error| {
                        MaintenanceSstateAdapterError::ProcessControl(error.to_string())
                    })?)
                }
            }
        } else {
            forced = true;
            child.kill().await.map_err(|error| {
                MaintenanceSstateAdapterError::ProcessControl(error.to_string())
            })?;
            Some(child.wait().await.map_err(|error| {
                MaintenanceSstateAdapterError::ProcessControl(error.to_string())
            })?)
        };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child.kill().await.map_err(|error| {
                MaintenanceSstateAdapterError::ProcessControl(error.to_string())
            })?;
            Some(child.wait().await.map_err(|error| {
                MaintenanceSstateAdapterError::ProcessControl(error.to_string())
            })?)
        };
        Ok((status, forced))
    }

    async fn kill_and_clear(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.clear_process_state();
    }

    fn clear_process_state(&mut self) {
        self.clear_process_state_preserving_events();
        self.terminal_pending.clear();
    }

    fn clear_process_state_preserving_events(&mut self) {
        self.id = None;
        self.child = None;
        self.output = None;
        self.streams_drained = true;
        self.started_pending = false;
        self.deadline = None;
        self.cancellation_requested = false;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
    }

    #[cfg(test)]
    pub(crate) fn lose_output_channel(&mut self) {
        self.output = None;
    }
}

impl Drop for MaintenanceSstateJobRunner {
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
