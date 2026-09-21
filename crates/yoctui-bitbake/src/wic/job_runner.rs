#[derive(Debug)]
enum WicPipeEvent {
    Output {
        stream: WicRunnerOutputStream,
        line: String,
        truncated: bool,
    },
    Failed(String),
}

async fn read_wic_output<R>(
    stream: R,
    kind: WicRunnerOutputStream,
    sender: tokio::sync::mpsc::Sender<WicPipeEvent>,
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
                let _ = sender.send(WicPipeEvent::Failed(error.to_string())).await;
                return;
            }
        };
        if buffer.is_empty() {
            if !bytes.is_empty() || truncated {
                let _ = sender
                    .send(WicPipeEvent::Output {
                        stream: kind,
                        line: output_text(&bytes),
                        truncated,
                    })
                    .await;
            }
            return;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(buffer.len());
        if !truncated {
            let remaining = MAX_WIC_LINE_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(WicPipeEvent::Output {
                    stream: kind,
                    line: output_text(&bytes),
                    truncated,
                })
                .await
                .is_err()
            {
                return;
            }
            bytes.clear();
            truncated = false;
        }
    }
}

pub struct WicJobRunner {
    build_dir: PathBuf,
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<WicPipeEvent>>,
    start_events_pending: u8,
    terminal_pending: VecDeque<WicRunnerEvent>,
    output_root: Option<PathBuf>,
    before: WicOutputSnapshot,
    cancellation_timeout: Duration,
    execution_timeout: Duration,
    started_at: Option<Instant>,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl WicJobRunner {
    pub fn new(build_dir: PathBuf) -> Self {
        Self {
            build_dir,
            child: None,
            output: None,
            start_events_pending: 0,
            terminal_pending: VecDeque::new(),
            output_root: None,
            before: BTreeMap::new(),
            cancellation_timeout: Duration::from_secs(5),
            execution_timeout: Duration::from_secs(60 * 60),
            started_at: None,
            cancellation_requested: false,
            #[cfg(unix)]
            process_group: None,
        }
    }

    pub fn with_cancellation_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_timeout = timeout;
        self
    }

    pub fn with_execution_timeout(mut self, timeout: Duration) -> Self {
        self.execution_timeout = timeout;
        self
    }

    pub async fn start(
        &mut self,
        command: WicCreateCommandSpec,
        output_directory: PathBuf,
    ) -> Result<(), WicAdapterError> {
        self.ensure_idle()?;
        let output_root = canonical_directory(&output_directory)?;
        let (before, _) = scan_outputs(&output_root)?;
        self.output_root = Some(output_root);
        self.before = before;
        self.spawn(&command.executable, &command.arguments)
    }

    pub async fn start_write(
        &mut self,
        inspector: &WicDeviceInspector,
        request: WicWriteRequest,
    ) -> Result<(), WicAdapterError> {
        self.ensure_idle()?;
        let command = inspector.command_for(&request).await?;
        self.output_root = None;
        self.before.clear();
        self.spawn(&command.executable, &command.arguments)
    }

    fn ensure_idle(&self) -> Result<(), WicAdapterError> {
        if self.child.is_some()
            || self.output.is_some()
            || self.start_events_pending > 0
            || !self.terminal_pending.is_empty()
        {
            Err(WicAdapterError::Busy)
        } else {
            Ok(())
        }
    }

    fn spawn(&mut self, executable: &Path, arguments: &[OsString]) -> Result<(), WicAdapterError> {
        if !self.build_dir.is_dir() {
            return Err(WicAdapterError::Spawn(format!(
                "build directory does not exist: {}",
                self.build_dir.display()
            )));
        }
        let mut process = Command::new(executable);
        process
            .args(arguments)
            .current_dir(&self.build_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        process.process_group(0);
        let mut child = process
            .spawn()
            .map_err(|error| WicAdapterError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| WicAdapterError::Spawn("stdout is unavailable".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| WicAdapterError::Spawn("stderr is unavailable".into()))?;
        let (sender, receiver) = tokio::sync::mpsc::channel(WIC_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_wic_output(
            stdout,
            WicRunnerOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_wic_output(
            stderr,
            WicRunnerOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.child = Some(child);
        self.output = Some(receiver);
        self.start_events_pending = 2;
        self.cancellation_requested = false;
        self.started_at = Some(Instant::now());
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<WicRunnerEvent, WicAdapterError> {
        if self.start_events_pending == 2 {
            self.start_events_pending = 1;
            return Ok(WicRunnerEvent::Starting);
        }
        if self.start_events_pending == 1 {
            self.start_events_pending = 0;
            return Ok(WicRunnerEvent::Started);
        }
        let remaining = self.remaining();
        if let Some(receiver) = self.output.as_mut() {
            match tokio::time::timeout(remaining, receiver.recv()).await {
                Err(_) => {
                    self.kill_and_clear().await;
                    return Ok(WicRunnerEvent::Failed {
                        message: "Wic operation timed out".into(),
                        exit_code: None,
                    });
                }
                Ok(Some(WicPipeEvent::Output {
                    stream,
                    line,
                    truncated,
                })) => {
                    return Ok(WicRunnerEvent::Output {
                        stream,
                        line,
                        truncated,
                    });
                }
                Ok(Some(WicPipeEvent::Failed(message))) => {
                    self.kill_and_clear().await;
                    return Ok(WicRunnerEvent::Lost { message });
                }
                Ok(None) => {
                    self.output = None;
                }
            }
        }
        if let Some(event) = self.terminal_pending.pop_front() {
            return Ok(event);
        }
        let remaining = self.remaining();
        let child = self.child.as_mut().ok_or(WicAdapterError::NotRunning)?;
        let status = match tokio::time::timeout(remaining, child.wait()).await {
            Ok(Ok(status)) => status,
            Ok(Err(error)) => {
                self.kill_and_clear().await;
                return Ok(WicRunnerEvent::Lost {
                    message: format!("Wic process wait failed: {error}"),
                });
            }
            Err(_) => {
                self.kill_and_clear().await;
                return Ok(WicRunnerEvent::Failed {
                    message: "Wic operation timed out".into(),
                    exit_code: None,
                });
            }
        };
        self.child = None;
        self.clear_process_state();
        if !status.success() {
            return Ok(WicRunnerEvent::Failed {
                message: "Wic operation exited unsuccessfully".into(),
                exit_code: status.code(),
            });
        }
        let (outputs, limitations) = if let Some(root) = self.output_root.take() {
            let (after, limitations) = scan_outputs(&root)?;
            let outputs = after
                .into_iter()
                .filter(|(path, identity)| self.before.get(path) != Some(identity))
                .map(|(path, (size_bytes, modified_nanoseconds))| WicOutput {
                    kind: classify_output(&path),
                    identity: WicOutputIdentity {
                        path,
                        size_bytes,
                        modified_unix_seconds: (modified_nanoseconds / 1_000_000_000) as u64,
                    },
                })
                .collect();
            (outputs, limitations)
        } else {
            (Vec::new(), Vec::new())
        };
        self.before.clear();
        Ok(WicRunnerEvent::Completed {
            exit_code: status.code().unwrap_or(0),
            outputs,
            limitations,
        })
    }

    pub async fn cancel(&mut self) -> Result<bool, WicAdapterError> {
        if self.cancellation_requested || self.child.is_none() {
            self.terminal_pending
                .push_back(WicRunnerEvent::CancellationRejected {
                    message: "no cancellable Wic process is active".into(),
                });
            return Ok(false);
        }
        self.cancellation_requested = true;
        let child = self.child.as_mut().expect("checked above");
        let mut forced = false;
        #[cfg(unix)]
        let status = if let Some(group) = self.process_group {
            if unsafe { libc::kill(-group, libc::SIGTERM) } != 0 {
                child
                    .start_kill()
                    .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?;
                forced = true;
            }
            match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                Ok(result) => {
                    result.map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?
                }
                Err(_) => {
                    let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
                    forced = true;
                    child
                        .wait()
                        .await
                        .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?
                }
            }
        } else {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?
        };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?
        };
        self.child = None;
        self.clear_process_state();
        self.terminal_pending.push_back(WicRunnerEvent::Cancelled {
            forced,
            exit_code: status.code(),
        });
        Ok(true)
    }

    async fn kill_and_clear(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.child = None;
        self.output = None;
        self.clear_process_state();
    }

    fn clear_process_state(&mut self) {
        self.cancellation_requested = false;
        self.started_at = None;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
    }

    fn remaining(&self) -> Duration {
        self.started_at
            .map(|started| self.execution_timeout.saturating_sub(started.elapsed()))
            .unwrap_or(self.execution_timeout)
    }
}

impl Drop for WicJobRunner {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(group) = self.process_group {
            let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
    }
}
