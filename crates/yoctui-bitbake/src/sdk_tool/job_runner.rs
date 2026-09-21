#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdkToolRunnerEvent {
    Started,
    Output {
        stream: SdkOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        exit_code: Option<i32>,
    },
    Failed {
        exit_code: Option<i32>,
    },
    Cancelled {
        forced: bool,
        exit_code: Option<i32>,
    },
    CancellationRejected {
        message: String,
    },
    TimedOut {
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        message: String,
    },
}

#[derive(Debug)]
enum SdkToolPipeEvent {
    Output {
        stream: SdkOutputStream,
        line: String,
        truncated: bool,
    },
    Failed {
        stream: SdkOutputStream,
        message: String,
    },
}

async fn read_sdk_tool_output<R>(
    stream: R,
    kind: SdkOutputStream,
    sender: tokio::sync::mpsc::Sender<SdkToolPipeEvent>,
) where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut bytes = Vec::new();
    let mut truncated = false;
    loop {
        let buffer = match reader.fill_buf().await {
            Ok(buffer) => buffer,
            Err(error) => {
                let _ = sender
                    .send(SdkToolPipeEvent::Failed {
                        stream: kind,
                        message: error.to_string(),
                    })
                    .await;
                break;
            }
        };
        if buffer.is_empty() {
            if !bytes.is_empty() || truncated {
                let _ = sender
                    .send(SdkToolPipeEvent::Output {
                        stream: kind,
                        line: output_text(&bytes),
                        truncated,
                    })
                    .await;
            }
            break;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(buffer.len());
        if !truncated {
            let remaining = MAX_SDK_TOOL_LINE_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(SdkToolPipeEvent::Output {
                    stream: kind,
                    line: output_text(&bytes),
                    truncated,
                })
                .await
                .is_err()
            {
                break;
            }
            bytes.clear();
            truncated = false;
        }
    }
}

pub struct SdkToolJobRunner {
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<SdkToolPipeEvent>>,
    streams_drained: bool,
    started_pending: bool,
    terminal_pending: VecDeque<SdkToolRunnerEvent>,
    cancellation_timeout: Duration,
    operation_timeout: Duration,
    deadline: Option<Instant>,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl Default for SdkToolJobRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl SdkToolJobRunner {
    pub fn new() -> Self {
        Self {
            child: None,
            output: None,
            streams_drained: true,
            started_pending: false,
            terminal_pending: VecDeque::new(),
            cancellation_timeout: Duration::from_secs(5),
            operation_timeout: SDK_TOOL_OPERATION_TIMEOUT,
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

    pub fn operation_timeout_due_within(&self, window: Duration) -> bool {
        self.deadline
            .is_some_and(|deadline| deadline <= Instant::now() + window)
    }

    pub async fn start(&mut self, command: SdkToolCommandSpec) -> Result<(), SdkToolAdapterError> {
        if self.child.is_some()
            || self.started_pending
            || !self.terminal_pending.is_empty()
            || self.output.is_some()
        {
            return Err(SdkToolAdapterError::Busy);
        }
        command.revalidate()?;
        let mut process = Command::new(&command.executable);
        process
            .args(&command.arguments)
            .current_dir(&command.current_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if command.clear_environment {
            process.env_clear();
        }
        process.envs(&command.environment);
        #[cfg(unix)]
        process.process_group(0);
        let mut child = spawn_sdk_tool_process(&mut process)
            .await
            .map_err(|error| SdkToolAdapterError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            #[cfg(unix)]
            {
                self.process_group = None;
            }
            return Err(SdkToolAdapterError::StreamUnavailable(
                SdkOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill().await;
            #[cfg(unix)]
            {
                self.process_group = None;
            }
            return Err(SdkToolAdapterError::StreamUnavailable(
                SdkOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(SDK_TOOL_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_sdk_tool_output(
            stdout,
            SdkOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_sdk_tool_output(
            stderr,
            SdkOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.child = Some(child);
        self.output = Some(receiver);
        self.streams_drained = false;
        self.started_pending = true;
        self.deadline = Some(Instant::now() + self.operation_timeout);
        self.cancellation_requested = false;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<SdkToolRunnerEvent, SdkToolAdapterError> {
        if self.started_pending {
            self.started_pending = false;
            return Ok(SdkToolRunnerEvent::Started);
        }
        if let Some(event) = self.terminal_pending.pop_front() {
            return Ok(event);
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            self.kill_and_clear().await;
            return Ok(SdkToolRunnerEvent::Lost {
                message: "SDK tool output event channel was lost".into(),
            });
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return self.timeout_active().await;
        }
        if let Some(receiver) = self.output.as_mut() {
            let deadline = self.deadline.ok_or(SdkToolAdapterError::NotRunning)?;
            let event = tokio::select! {
                event = receiver.recv() => Some(event),
                _ = tokio::time::sleep_until(deadline) => None,
            };
            match event {
                Some(Some(SdkToolPipeEvent::Output {
                    stream,
                    line,
                    truncated,
                })) => {
                    return Ok(SdkToolRunnerEvent::Output {
                        stream,
                        line,
                        truncated,
                    });
                }
                Some(Some(SdkToolPipeEvent::Failed { stream, message })) => {
                    self.kill_and_clear().await;
                    return Ok(SdkToolRunnerEvent::Lost {
                        message: format!("{stream:?} stream failed: {message}"),
                    });
                }
                Some(None) => {
                    self.output = None;
                    self.streams_drained = true;
                }
                None => return self.timeout_active().await,
            }
        }
        let deadline = self.deadline.ok_or(SdkToolAdapterError::NotRunning)?;
        let status = {
            let child = self.child.as_mut().ok_or(SdkToolAdapterError::NotRunning)?;
            match tokio::time::timeout_at(deadline, child.wait()).await {
                Ok(Ok(status)) => status,
                Ok(Err(error)) => {
                    self.kill_and_clear().await;
                    return Ok(SdkToolRunnerEvent::Lost {
                        message: format!("SDK tool process wait failed: {error}"),
                    });
                }
                Err(_) => return self.timeout_active().await,
            }
        };
        self.clear_process_state();
        if status.success() {
            Ok(SdkToolRunnerEvent::Completed {
                exit_code: status.code(),
            })
        } else {
            Ok(SdkToolRunnerEvent::Failed {
                exit_code: status.code(),
            })
        }
    }

    pub async fn cancel(&mut self) -> Result<bool, SdkToolAdapterError> {
        if self.cancellation_requested || self.child.is_none() {
            self.terminal_pending
                .push_back(SdkToolRunnerEvent::CancellationRejected {
                    message: "no cancellable SDK tool process is active".into(),
                });
            return Ok(false);
        }
        self.cancellation_requested = true;
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state();
        self.terminal_pending
            .push_back(SdkToolRunnerEvent::Cancelled {
                forced,
                exit_code: status.and_then(|status| status.code()),
            });
        Ok(true)
    }

    async fn timeout_active(&mut self) -> Result<SdkToolRunnerEvent, SdkToolAdapterError> {
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state();
        Ok(SdkToolRunnerEvent::TimedOut {
            forced,
            exit_code: status.and_then(|status| status.code()),
        })
    }

    async fn terminate_active(
        &mut self,
    ) -> Result<(Option<std::process::ExitStatus>, bool), SdkToolAdapterError> {
        let Some(child) = self.child.as_mut() else {
            return Ok((None, false));
        };
        let mut forced = false;
        #[cfg(unix)]
        let status = if let Some(process_group) = self.process_group {
            // SAFETY: the negative PID targets only the process group created for this child.
            if unsafe { libc::kill(-process_group, libc::SIGTERM) } != 0 {
                child
                    .start_kill()
                    .map_err(|error| SdkToolAdapterError::ProcessControl(error.to_string()))?;
                forced = true;
            }
            match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                Ok(result) => Some(
                    result
                        .map_err(|error| SdkToolAdapterError::ProcessControl(error.to_string()))?,
                ),
                Err(_) => {
                    // SAFETY: same child-owned process group as the graceful signal.
                    let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                    forced = true;
                    Some(
                        child.wait().await.map_err(|error| {
                            SdkToolAdapterError::ProcessControl(error.to_string())
                        })?,
                    )
                }
            }
        } else {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| SdkToolAdapterError::ProcessControl(error.to_string()))?;
            Some(
                child
                    .wait()
                    .await
                    .map_err(|error| SdkToolAdapterError::ProcessControl(error.to_string()))?,
            )
        };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| SdkToolAdapterError::ProcessControl(error.to_string()))?;
            Some(
                child
                    .wait()
                    .await
                    .map_err(|error| SdkToolAdapterError::ProcessControl(error.to_string()))?,
            )
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
        self.child = None;
        self.output = None;
        self.streams_drained = true;
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

impl Drop for SdkToolJobRunner {
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
