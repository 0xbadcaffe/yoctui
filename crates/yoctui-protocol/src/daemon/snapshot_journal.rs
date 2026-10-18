fn decimal_width(value: u64) -> usize {
    value.checked_ilog10().map_or(1, |log| log as usize + 1)
}

fn terminal_screen_sizes(
    snapshot: &DaemonSnapshot,
) -> Result<std::collections::HashMap<PtySessionId, usize>, DaemonSnapshotError> {
    snapshot
        .pty_screens
        .iter()
        .map(|screen| Ok((screen.session_id, serde_json::to_vec(screen)?.len())))
        .collect()
}

/// Single-owner snapshot/event journal. Calling `synchronize` while holding the
/// daemon state's lock establishes the snapshot/replay watermark before a
/// client is added to the live subscriber set.
#[derive(Debug, Clone)]
pub struct DaemonSnapshotJournal {
    snapshot: DaemonSnapshot,
    events: VecDeque<SequencedEvent>,
    event_sizes: VecDeque<usize>,
    retained_event_bytes: usize,
    limits: DaemonSnapshotLimits,
    snapshot_bytes_upper_bound: usize,
    published_events: u64,
    published_event_bytes: u64,
    snapshot_serializations: u64,
    // Exact leaf sizes let a screen replace itself without re-encoding the
    // entire retained build/other terminals to prove the snapshot byte bound.
    pty_screen_bytes: std::collections::HashMap<PtySessionId, usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonIpcMetrics {
    pub published_events: u64,
    pub published_event_bytes: u64,
    pub snapshot_serializations: u64,
    pub snapshot_bytes_upper_bound: usize,
}

impl DaemonSnapshotJournal {
    pub fn new(
        mut snapshot: DaemonSnapshot,
        limits: DaemonSnapshotLimits,
    ) -> Result<Self, DaemonSnapshotError> {
        let limits = limits.validate()?;
        let limits = DaemonSnapshotLimits {
            snapshot_bytes: limits.snapshot_bytes.min(MAX_SNAPSHOT_BYTES),
            ..limits
        };
        if let Some(compatibility) = &snapshot.compatibility {
            compatibility.validate()?;
        }
        if snapshot.raw_executions.len() > MAX_RAW_EXECUTION_REQUESTS {
            return Err(DaemonSnapshotError::TooManyRawExecutions);
        }
        let mut raw_request_ids = std::collections::BTreeSet::new();
        for execution in &snapshot.raw_executions {
            execution.validate()?;
            if !raw_request_ids.insert(&execution.request.request_id) {
                return Err(DaemonSnapshotError::RawExecution(
                    RawExecutionProtocolError::DuplicateRequest,
                ));
            }
        }
        validate_raw_history_records(&snapshot.raw_history)?;
        for screen in &snapshot.pty_screens {
            validate_pty_screen(screen)?;
        }
        let mut snapshot_serializations = 0;
        let snapshot_bytes = compact_snapshot(
            &mut snapshot,
            limits.snapshot_bytes,
            &mut snapshot_serializations,
        )?;
        Ok(Self {
            pty_screen_bytes: terminal_screen_sizes(&snapshot)?,
            snapshot,
            events: VecDeque::new(),
            event_sizes: VecDeque::new(),
            retained_event_bytes: 0,
            limits,
            snapshot_bytes_upper_bound: snapshot_bytes,
            published_events: 0,
            published_event_bytes: 0,
            snapshot_serializations,
        })
    }

    pub fn snapshot(&self) -> &DaemonSnapshot {
        &self.snapshot
    }

    pub fn retained_event_capacity(&self) -> usize {
        self.limits.retained_events
    }

    pub fn ipc_metrics(&self) -> DaemonIpcMetrics {
        DaemonIpcMetrics {
            published_events: self.published_events,
            published_event_bytes: self.published_event_bytes,
            snapshot_serializations: self.snapshot_serializations,
            snapshot_bytes_upper_bound: self.snapshot_bytes_upper_bound,
        }
    }

    pub fn publish(
        &mut self,
        mut event: DaemonEvent,
    ) -> Result<SequencedEvent, DaemonSnapshotError> {
        preserve_build_timing(&self.snapshot, &mut event);
        let sequence = self
            .snapshot
            .sequence
            .checked_add(1)
            .ok_or(DaemonSnapshotError::SequenceExhausted)?;
        let generation = self
            .snapshot
            .generation
            .checked_add(1)
            .ok_or(DaemonSnapshotError::GenerationExhausted)?;
        let mut sequenced = SequencedEvent {
            sequence,
            generation,
            event,
        };
        let mut event_bytes = serde_json::to_vec(&sequenced)?.len();
        if event_bytes > MAX_SNAPSHOT_BYTES
            && let DaemonEvent::RawExecutionChanged(execution) = &mut sequenced.event
        {
            // JSON escaping can expand a valid retained-output payload beyond
            // the wire limit. Bound the event too, without losing the result,
            // stream sequence, or truthful truncation counters.
            execution.validate()?;
            let wrapper = event_bytes - serde_json::to_vec(&*execution)?.len();
            while event_bytes > MAX_SNAPSHOT_BYTES {
                let mut needed = event_bytes - MAX_SNAPSHOT_BYTES;
                trim_raw_output(&mut execution.stdout, &mut needed)?;
                trim_raw_output(&mut execution.stderr, &mut needed)?;
                let compacted = serde_json::to_vec(&*execution)?.len() + wrapper;
                if compacted >= event_bytes {
                    break;
                }
                event_bytes = compacted;
            }
            event_bytes = serde_json::to_vec(&sequenced)?.len();
        }
        if event_bytes > MAX_SNAPSHOT_BYTES {
            return Err(DaemonSnapshotError::EventTooLarge {
                actual: event_bytes,
                maximum: MAX_SNAPSHOT_BYTES,
            });
        }
        let counter_growth = decimal_width(sequence) + decimal_width(generation)
            - decimal_width(self.snapshot.sequence)
            - decimal_width(self.snapshot.generation);
        let incoming_screen = if let DaemonEvent::PtyScreen(screen) = &sequenced.event {
            Some((screen.session_id, serde_json::to_vec(screen)?.len()))
        } else {
            None
        };
        let conservative_bytes = match &sequenced.event {
            // These bytes are replayed but not retained in the snapshot.
            DaemonEvent::PtyOutput { .. } => self
                .snapshot_bytes_upper_bound
                .saturating_add(counter_growth),
            DaemonEvent::PtyScreen(_) => {
                let (id, bytes) = incoming_screen.expect("screen size was measured");
                let previous = self.pty_screen_bytes.get(&id).copied();
                let comma =
                    usize::from(previous.is_none() && !self.snapshot.pty_screens.is_empty());
                self.snapshot_bytes_upper_bound
                    .saturating_sub(previous.unwrap_or(0))
                    .saturating_add(bytes)
                    .saturating_add(counter_growth + comma)
            }
            _ => self.snapshot_bytes_upper_bound.saturating_add(event_bytes),
        };
        let snapshot_bytes_upper_bound = if conservative_bytes <= self.limits.snapshot_bytes
            && matches!(
                &sequenced.event,
                DaemonEvent::Build(_)
                    | DaemonEvent::Log(_)
                    | DaemonEvent::JobChanged(_)
                    | DaemonEvent::Telemetry(_)
                    | DaemonEvent::PtyOutput { .. }
                    | DaemonEvent::PtyChanged(_)
                    | DaemonEvent::PtyRemoved { .. }
                    | DaemonEvent::PtyScreen(_)
            ) {
            // Build, log, job, telemetry and PTY metadata/output reduction cannot reject a payload
            // after the frame and sequence checks above. Telemetry only advances
            // the snapshot counters; its encoded event also covers their growth.
            // PtyScreen validates its complete payload BEFORE mutating state.
            // Its measured leaf-size delta covers replacement, the array comma,
            // and counter growth; no other terminal/build data needs cloning.
            // Apply these recurring records in
            // place while the conservative size ledger proves that the
            // resulting snapshot remains bounded; validation-sensitive event
            // variants retain the transactional clone below.
            apply_sequenced_event(&mut self.snapshot, &sequenced)?;
            if let Some((id, bytes)) = incoming_screen {
                self.pty_screen_bytes.insert(id, bytes);
            }
            while self.snapshot.recent_logs.len() > self.limits.recent_logs {
                self.snapshot.recent_logs.remove(0);
            }
            conservative_bytes
        } else {
            let mut candidate = self.snapshot.clone();
            apply_sequenced_event(&mut candidate, &sequenced)?;
            while candidate.recent_logs.len() > self.limits.recent_logs {
                candidate.recent_logs.remove(0);
            }
            let encoded_bytes = compact_snapshot(
                &mut candidate,
                self.limits.snapshot_bytes,
                &mut self.snapshot_serializations,
            )?;
            let screen_bytes = terminal_screen_sizes(&candidate)?;
            self.snapshot = candidate;
            self.pty_screen_bytes = screen_bytes;
            encoded_bytes
        };
        self.snapshot_bytes_upper_bound = snapshot_bytes_upper_bound;
        if let DaemonEvent::PtyRemoved { session_id } = &sequenced.event {
            self.pty_screen_bytes.remove(session_id);
        }
        self.published_events = self.published_events.saturating_add(1);
        self.published_event_bytes = self
            .published_event_bytes
            .saturating_add(u64::try_from(event_bytes).unwrap_or(u64::MAX));
        self.events.push_back(sequenced.clone());
        self.event_sizes.push_back(event_bytes);
        self.retained_event_bytes += event_bytes;
        while self.events.len() > self.limits.retained_events
            || self.retained_event_bytes > MAX_RETAINED_EVENT_BYTES
        {
            self.events.pop_front();
            self.retained_event_bytes -= self.event_sizes.pop_front().expect("one size per event");
        }
        Ok(sequenced)
    }

    pub fn synchronize(&self, resume: Option<ResumeCursor>) -> DaemonSnapshotSync {
        self.synchronize_with_limit(resume, None)
    }

    pub fn synchronize_bounded(
        &self,
        resume: ResumeCursor,
        maximum_events: usize,
    ) -> DaemonSnapshotSync {
        assert!(
            maximum_events > 0,
            "bounded replay requires an event budget"
        );
        self.synchronize_with_limit(Some(resume), Some(maximum_events))
    }

    fn synchronize_with_limit(
        &self,
        resume: Option<ResumeCursor>,
        maximum_events: Option<usize>,
    ) -> DaemonSnapshotSync {
        let Some(cursor) = resume else {
            return DaemonSnapshotSync::Replace {
                snapshot: Box::new(self.snapshot.clone()),
                reason: SnapshotReplacementReason::InitialAttach,
            };
        };
        if cursor.daemon_instance_id != self.snapshot.daemon_instance_id {
            return DaemonSnapshotSync::Replace {
                snapshot: Box::new(self.snapshot.clone()),
                reason: SnapshotReplacementReason::DaemonInstanceChanged,
            };
        }
        if cursor.last_sequence > self.snapshot.sequence {
            return DaemonSnapshotSync::Replace {
                snapshot: Box::new(self.snapshot.clone()),
                reason: SnapshotReplacementReason::CursorAhead,
            };
        }
        let first_retained = self
            .events
            .front()
            .map(|event| event.sequence)
            .unwrap_or_else(|| self.snapshot.sequence.saturating_add(1));
        if cursor.last_sequence.saturating_add(1) < first_retained {
            return DaemonSnapshotSync::Replace {
                snapshot: Box::new(self.snapshot.clone()),
                reason: SnapshotReplacementReason::HistoryExpired,
            };
        }
        let events = self
            .events
            .iter()
            .filter(|event| event.sequence > cursor.last_sequence)
            .take(maximum_events.unwrap_or(usize::MAX))
            .cloned()
            .collect::<Vec<_>>();
        let replayed_through = events
            .last()
            .map_or(cursor.last_sequence, |event| event.sequence);
        DaemonSnapshotSync::Replay {
            events,
            replayed_through,
        }
    }
}
