#[derive(Debug)]
enum QemuPipeEvent {
    Output {
        stream: QemuRunnerOutputStream,
        line: String,
        truncated: bool,
    },
    Failed {
        stream: QemuRunnerOutputStream,
        message: String,
    },
}

async fn read_qemu_output<R>(
    stream: R,
    kind: QemuRunnerOutputStream,
    sender: tokio::sync::mpsc::Sender<QemuPipeEvent>,
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
                    .send(QemuPipeEvent::Failed {
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
                    .send(QemuPipeEvent::Output {
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
            let remaining = MAX_QEMU_LINE_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(QemuPipeEvent::Output {
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

pub struct QemuJobRunner {
    build_dir: PathBuf,
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<QemuPipeEvent>>,
    streams_drained: bool,
    start_events_pending: u8,
    terminal_pending: VecDeque<QemuRunnerEvent>,
    cancellation_timeout: Duration,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl QemuJobRunner {
    pub fn new(build_dir: PathBuf) -> Self {
        Self {
            build_dir,
            child: None,
            output: None,
            streams_drained: true,
            start_events_pending: 0,
            terminal_pending: VecDeque::new(),
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

    pub async fn start(&mut self, command: QemuCommandSpec) -> Result<(), QemuAdapterError> {
        if self.child.is_some()
            || self.start_events_pending > 0
            || !self.terminal_pending.is_empty()
            || self.output.is_some()
        {
            return Err(QemuAdapterError::Busy);
        }
        if !self.build_dir.is_dir() {
            return Err(QemuAdapterError::Spawn(format!(
                "build directory does not exist: {}",
                self.build_dir.display()
            )));
        }
        let mut process = Command::new(&command.executable);
        process
            .args(&command.arguments)
            .current_dir(&self.build_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        process.process_group(0);
        let mut child = process.spawn().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                QemuAdapterError::MissingExecutable(command.executable.clone())
            } else {
                QemuAdapterError::Spawn(error.to_string())
            }
        })?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            return Err(QemuAdapterError::StreamUnavailable(
                QemuRunnerOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill().await;
            return Err(QemuAdapterError::StreamUnavailable(
                QemuRunnerOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(QEMU_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_qemu_output(
            stdout,
            QemuRunnerOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_qemu_output(
            stderr,
            QemuRunnerOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.child = Some(child);
        self.output = Some(receiver);
        self.streams_drained = false;
        self.start_events_pending = 2;
        self.cancellation_requested = false;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<QemuRunnerEvent, QemuAdapterError> {
        if self.start_events_pending == 2 {
            self.start_events_pending = 1;
            return Ok(QemuRunnerEvent::Starting);
        }
        if self.start_events_pending == 1 {
            self.start_events_pending = 0;
            return Ok(QemuRunnerEvent::Started);
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            self.kill_and_clear().await;
            return Ok(QemuRunnerEvent::Lost {
                message: "runqemu output event channel was lost".into(),
            });
        }
        if let Some(receiver) = self.output.as_mut() {
            match receiver.recv().await {
                Some(QemuPipeEvent::Output {
                    stream,
                    line,
                    truncated,
                }) => {
                    return Ok(QemuRunnerEvent::Output {
                        stream,
                        line,
                        truncated,
                    });
                }
                Some(QemuPipeEvent::Failed { stream, message }) => {
                    self.kill_and_clear().await;
                    return Ok(QemuRunnerEvent::Lost {
                        message: format!("{stream:?} stream failed: {message}"),
                    });
                }
                None => {
                    self.output = None;
                    self.streams_drained = true;
                }
            }
        }
        if let Some(event) = self.terminal_pending.pop_front() {
            return Ok(event);
        }
        let Some(child) = self.child.as_mut() else {
            return Err(QemuAdapterError::NotRunning);
        };
        let status = match child.wait().await {
            Ok(status) => status,
            Err(error) => {
                self.child = None;
                self.clear_process_state();
                return Ok(QemuRunnerEvent::Lost {
                    message: format!("runqemu process wait failed: {error}"),
                });
            }
        };
        self.child = None;
        self.clear_process_state();
        let exit_code = status.code();
        if status.success() {
            Ok(QemuRunnerEvent::Completed {
                exit_code: exit_code.unwrap_or(0),
            })
        } else {
            Ok(QemuRunnerEvent::Failed {
                message: "runqemu exited unsuccessfully".into(),
                exit_code,
            })
        }
    }

    pub async fn cancel(&mut self) -> Result<bool, QemuAdapterError> {
        if self.cancellation_requested || self.child.is_none() {
            self.terminal_pending
                .push_back(QemuRunnerEvent::CancellationRejected {
                    message: "no cancellable runqemu process is active".into(),
                });
            return Ok(false);
        }
        self.cancellation_requested = true;
        let child = self.child.as_mut().expect("checked above");
        let mut forced = false;
        #[cfg(unix)]
        let status = if let Some(process_group) = self.process_group {
            // SAFETY: this negative PID targets only the group created for this child.
            if unsafe { libc::kill(-process_group, libc::SIGTERM) } != 0 {
                child
                    .start_kill()
                    .map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?;
                forced = true;
            }
            match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                Ok(result) => {
                    result.map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?
                }
                Err(_) => {
                    // SAFETY: same child-owned process group as the graceful signal.
                    let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                    forced = true;
                    child
                        .wait()
                        .await
                        .map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?
                }
            }
        } else {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?
        };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| QemuAdapterError::ProcessControl(error.to_string()))?
        };
        self.child = None;
        self.clear_process_state();
        self.terminal_pending.push_back(QemuRunnerEvent::Cancelled {
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
        self.streams_drained = true;
        self.clear_process_state();
    }

    fn clear_process_state(&mut self) {
        self.cancellation_requested = false;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
    }
}

impl Drop for QemuJobRunner {
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
