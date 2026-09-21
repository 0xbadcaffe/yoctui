impl PtySession {
    pub fn new(spec: PtySessionSpec, process_group: i32) -> Result<Self, PtySessionError> {
        validate_spec(&spec)?;
        if process_group <= 0 {
            return Err(PtySessionError::InvalidProcessGroup(process_group));
        }
        Ok(Self {
            id: spec.id,
            name: spec.name,
            kind: spec.kind,
            cwd: spec.cwd,
            command: spec.command,
            lifecycle: PtySessionLifecycle::Starting,
            dimensions: spec.dimensions,
            attached_clients: BTreeSet::new(),
            writer: None,
            writer_epoch: 0,
            process_group: Some(process_group),
            scrollback: PtyScrollbackMetadata::default(),
            exit_status: None,
            restartable: spec.restartable,
            workspace: spec.workspace,
        })
    }

    pub fn apply(&mut self, action: PtySessionAction) -> Result<(), PtySessionError> {
        match action {
            PtySessionAction::MarkRunning => {
                self.require(PtySessionLifecycle::Starting)?;
                self.lifecycle = PtySessionLifecycle::Running;
            }
            PtySessionAction::Attach(client) => {
                self.attached_clients.insert(client);
            }
            PtySessionAction::Detach(client) => {
                if !self.attached_clients.remove(&client) {
                    return Err(PtySessionError::ClientNotAttached(client));
                }
                if self.writer.is_some_and(|writer| writer.client == client) {
                    self.release_writer()?;
                }
            }
            PtySessionAction::TakeControl {
                client,
                expected_epoch,
            } => {
                if !self.attached_clients.contains(&client) {
                    return Err(PtySessionError::ClientNotAttached(client));
                }
                if self.lifecycle != PtySessionLifecycle::Running {
                    return Err(PtySessionError::NotRunning(self.lifecycle));
                }
                if self.writer.is_some() {
                    return Err(PtySessionError::WriterBusy);
                }
                if expected_epoch != self.writer_epoch {
                    return Err(PtySessionError::StaleWriterEpoch {
                        expected: self.writer_epoch,
                        actual: expected_epoch,
                    });
                }
                self.writer_epoch = self
                    .writer_epoch
                    .checked_add(1)
                    .ok_or(PtySessionError::WriterEpochExhausted)?;
                self.writer = Some(PtyWriterLease {
                    client,
                    epoch: self.writer_epoch,
                });
            }
            PtySessionAction::ReleaseControl {
                client,
                expected_epoch,
            } => self
                .require_writer(client, expected_epoch)
                .and_then(|_| self.release_writer())?,
            PtySessionAction::Resize {
                client,
                writer_epoch,
                dimensions,
            } => {
                self.require_writer(client, writer_epoch)?;
                self.dimensions = dimensions.validate()?;
            }
            PtySessionAction::AdvanceScrollback(next) => {
                if next.next_sequence < self.scrollback.next_sequence
                    || next.first_sequence > next.next_sequence
                    || next.retained_lines > next.retained_cells
                    || next.retained_cells > next.retained_bytes.saturating_mul(4)
                    || next.retained_lines > MAX_PTY_SCROLLBACK_LINES
                    || next.retained_cells > MAX_PTY_SCROLLBACK_CELLS
                    || next.retained_bytes > MAX_PTY_SCROLLBACK_BYTES
                    || next.dropped_lines < self.scrollback.dropped_lines
                {
                    return Err(PtySessionError::InvalidScrollback);
                }
                self.scrollback = next;
            }
            PtySessionAction::BeginTermination => {
                self.require(PtySessionLifecycle::Running)?;
                self.lifecycle = PtySessionLifecycle::Terminating;
            }
            PtySessionAction::Exit(status) => {
                if !matches!(
                    self.lifecycle,
                    PtySessionLifecycle::Starting
                        | PtySessionLifecycle::Running
                        | PtySessionLifecycle::Terminating
                ) {
                    return Err(PtySessionError::InvalidTransition(self.lifecycle));
                }
                self.lifecycle = PtySessionLifecycle::Exited;
                self.exit_status = Some(status);
                self.clear_live_ownership()?;
            }
            PtySessionAction::MarkLost => {
                if self.lifecycle.is_terminal() {
                    return Err(PtySessionError::InvalidTransition(self.lifecycle));
                }
                self.lifecycle = PtySessionLifecycle::Lost;
                self.exit_status = None;
                self.clear_live_ownership()?;
            }
            PtySessionAction::Rename(name) => {
                validate_name(&name)?;
                self.name = name;
            }
        }
        Ok(())
    }

    fn require(&self, lifecycle: PtySessionLifecycle) -> Result<(), PtySessionError> {
        if self.lifecycle != lifecycle {
            return Err(PtySessionError::InvalidTransition(self.lifecycle));
        }
        Ok(())
    }

    fn require_writer(&self, client: PtyClientId, epoch: u64) -> Result<(), PtySessionError> {
        if self.writer != Some(PtyWriterLease { client, epoch }) {
            return Err(PtySessionError::NotWriter);
        }
        Ok(())
    }

    fn release_writer(&mut self) -> Result<(), PtySessionError> {
        self.writer = None;
        self.writer_epoch = self
            .writer_epoch
            .checked_add(1)
            .ok_or(PtySessionError::WriterEpochExhausted)?;
        Ok(())
    }

    fn clear_live_ownership(&mut self) -> Result<(), PtySessionError> {
        if self.writer.is_some() {
            self.release_writer()?;
        }
        self.attached_clients.clear();
        self.process_group = None;
        Ok(())
    }
}
