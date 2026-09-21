pub struct RawJobRunner {
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<RawJobPipeEvent>>,
    streams_drained: bool,
    started_pending: bool,
    terminal_pending: Option<RawJobRunnerEvent>,
    stdout_stream: Option<RawStreamId>,
    stderr_stream: Option<RawStreamId>,
    stdout_sequence: u64,
    stderr_sequence: u64,
    deadline: Option<Instant>,
    operation_timeout: Duration,
    cancellation_timeout: Duration,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl Default for RawJobRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl RawJobRunner {
    pub fn new() -> Self {
        Self {
            child: None,
            output: None,
            streams_drained: true,
            started_pending: false,
            terminal_pending: None,
            stdout_stream: None,
            stderr_stream: None,
            stdout_sequence: 1,
            stderr_sequence: 1,
            deadline: None,
            operation_timeout: RAW_JOB_DEFAULT_TIMEOUT,
            cancellation_timeout: RAW_JOB_DEFAULT_CANCELLATION_TIMEOUT,
            cancellation_requested: false,
            #[cfg(unix)]
            process_group: None,
        }
    }

    pub fn with_operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    pub fn with_cancellation_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_timeout = timeout;
        self
    }

    pub fn is_active(&self) -> bool {
        self.child.is_some()
    }

    pub async fn start(&mut self, spec: RawJobCommandSpec) -> Result<(), RawJobRunnerError> {
        if self.child.is_some()
            || self.output.is_some()
            || self.started_pending
            || self.terminal_pending.is_some()
        {
            return Err(RawJobRunnerError::Busy);
        }
        validate_directory(&spec.current_directory)
            .map_err(|error| RawJobRunnerError::Authorization(error.to_string()))?;
        validate_executable(&spec.executable)
            .map_err(|error| RawJobRunnerError::Authorization(error.to_string()))?;
        let mut process = Command::new(&spec.executable);
        process
            .args(&spec.arguments)
            .current_dir(&spec.current_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        process.process_group(0);
        let mut child = spawn_raw_job_process(&mut process)
            .await
            .map_err(|error| RawJobRunnerError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(RawJobRunnerError::StreamUnavailable(
                RawOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(RawJobRunnerError::StreamUnavailable(
                RawOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(RAW_JOB_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_raw_job_output(
            stdout,
            RawOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_raw_job_output(
            stderr,
            RawOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.stdout_stream = Some(spec.stdout_stream);
        self.stderr_stream = Some(spec.stderr_stream);
        self.stdout_sequence = 1;
        self.stderr_sequence = 1;
        self.child = Some(child);
        self.output = Some(receiver);
        self.streams_drained = false;
        self.started_pending = true;
        self.deadline = Some(Instant::now() + self.operation_timeout);
        self.cancellation_requested = false;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<RawJobRunnerEvent, RawJobRunnerError> {
        if self.started_pending {
            self.started_pending = false;
            return Ok(RawJobRunnerEvent::Started);
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            self.terminate_now().await;
            return Ok(RawJobRunnerEvent::Lost {
                message: "Raw job output event channel was lost".into(),
            });
        }
        if let Some(receiver) = self.output.as_mut() {
            let event = if let Some(deadline) = self.deadline {
                match tokio::time::timeout_at(deadline, receiver.recv()).await {
                    Ok(event) => event,
                    Err(_) => {
                        let (forced, exit_code) = self.terminate_with_grace(false).await?;
                        return Ok(RawJobRunnerEvent::TimedOut { forced, exit_code });
                    }
                }
            } else {
                receiver.recv().await
            };
            match event {
                Some(RawJobPipeEvent::Output {
                    stream,
                    text,
                    truncated_bytes,
                }) => return self.output_event(stream, text, truncated_bytes),
                Some(RawJobPipeEvent::Failed { stream, message }) => {
                    self.terminate_now().await;
                    return Ok(RawJobRunnerEvent::Lost {
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
            return Err(RawJobRunnerError::NotRunning);
        };
        let status = match child.wait().await {
            Ok(status) => status,
            Err(error) => {
                let message = format!("Raw job wait failed: {error}");
                self.clear_live();
                return Ok(RawJobRunnerEvent::Lost { message });
            }
        };
        self.clear_live();
        match status.code() {
            Some(0) => Ok(RawJobRunnerEvent::Completed { exit_code: 0 }),
            exit_code => Ok(RawJobRunnerEvent::Failed {
                exit_code,
                message: "Raw BitBake command exited unsuccessfully".into(),
            }),
        }
    }

    fn output_event(
        &mut self,
        stream: RawOutputStream,
        text: String,
        truncated_bytes: u64,
    ) -> Result<RawJobRunnerEvent, RawJobRunnerError> {
        let (stream_id, sequence) = match stream {
            RawOutputStream::Stdout => {
                let sequence = self.stdout_sequence;
                self.stdout_sequence = self
                    .stdout_sequence
                    .checked_add(1)
                    .ok_or(RawJobRunnerError::SequenceExhausted)?;
                (self.stdout_stream.clone(), sequence)
            }
            RawOutputStream::Stderr => {
                let sequence = self.stderr_sequence;
                self.stderr_sequence = self
                    .stderr_sequence
                    .checked_add(1)
                    .ok_or(RawJobRunnerError::SequenceExhausted)?;
                (self.stderr_stream.clone(), sequence)
            }
        };
        let chunk = RawOutputChunk {
            stream_id: stream_id.ok_or(RawJobRunnerError::NotRunning)?,
            stream,
            sequence,
            text,
            truncated_bytes,
            dropped_lines: 0,
        };
        chunk
            .validate()
            .map_err(|error| RawJobRunnerError::Output(error.to_string()))?;
        Ok(RawJobRunnerEvent::Output(chunk))
    }

    pub async fn cancel(&mut self) -> Result<bool, RawJobRunnerError> {
        if self.cancellation_requested || self.child.is_none() {
            return Ok(false);
        }
        self.cancellation_requested = true;
        let (forced, exit_code) = self.terminate_with_grace(true).await?;
        self.terminal_pending = Some(RawJobRunnerEvent::Cancelled { forced, exit_code });
        Ok(true)
    }

    async fn terminate_with_grace(
        &mut self,
        retain_buffered_output: bool,
    ) -> Result<(bool, Option<i32>), RawJobRunnerError> {
        let Some(child) = self.child.as_mut() else {
            return Err(RawJobRunnerError::NotRunning);
        };
        let mut forced = false;
        #[cfg(unix)]
        let status = if let Some(process_group) = self.process_group {
            // SAFETY: this process group is the child PID created by `process_group(0)`.
            if unsafe { libc::kill(-process_group, libc::SIGTERM) } != 0 {
                child
                    .start_kill()
                    .map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?;
                forced = true;
            }
            match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                Ok(status) => {
                    status.map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?
                }
                Err(_) => {
                    // SAFETY: this is the same child-owned process group.
                    let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                    forced = true;
                    child
                        .wait()
                        .await
                        .map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?
                }
            }
        } else {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?
        };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| RawJobRunnerError::ProcessControl(error.to_string()))?
        };
        let exit_code = status.code();
        if retain_buffered_output {
            self.clear_process();
        } else {
            self.clear_live();
        }
        Ok((forced, exit_code))
    }

    async fn terminate_now(&mut self) {
        #[cfg(unix)]
        if let Some(process_group) = self.process_group {
            // SAFETY: this process group is the child PID created by `process_group(0)`.
            let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
        self.clear_live();
    }

    fn clear_live(&mut self) {
        self.clear_process();
        self.output = None;
        self.streams_drained = true;
    }

    fn clear_process(&mut self) {
        self.child = None;
        self.deadline = None;
        self.cancellation_requested = false;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
    }

    #[cfg(test)]
    fn lose_output_channel(&mut self) {
        self.output = None;
    }
}

impl Drop for RawJobRunner {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(process_group) = self.process_group {
            // SAFETY: this process group is the child PID created by `start`.
            let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
    }
}

async fn spawn_raw_job_process(process: &mut Command) -> io::Result<Child> {
    for attempt in 1..=RAW_JOB_SPAWN_ATTEMPTS {
        match process.spawn() {
            Ok(child) => return Ok(child),
            Err(error) if attempt < RAW_JOB_SPAWN_ATTEMPTS && transient_spawn_error(&error) => {
                tokio::time::sleep(RAW_JOB_SPAWN_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("bounded Raw job spawn loop always returns")
}

#[cfg(unix)]
fn transient_spawn_error(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ETXTBSY)
}

#[cfg(not(unix))]
fn transient_spawn_error(_error: &io::Error) -> bool {
    false
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RawJobRunnerError {
    #[error("a Raw process or unconsumed terminal event is already active")]
    Busy,
    #[error("Raw command authorization became stale: {0}")]
    Authorization(String),
    #[error("could not start Raw command: {0}")]
    Spawn(String),
    #[error("Raw process stream is unavailable: {0:?}")]
    StreamUnavailable(RawOutputStream),
    #[error("Raw runner is not active")]
    NotRunning,
    #[error("Raw process control failed: {0}")]
    ProcessControl(String),
    #[error("Raw output sequence space is exhausted")]
    SequenceExhausted,
    #[error("Raw output was invalid: {0}")]
    Output(String),
}
