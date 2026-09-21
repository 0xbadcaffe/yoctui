/// Single-owner snapshot/event journal. Calling `synchronize` while holding the
/// daemon state's lock establishes the snapshot/replay watermark before a
/// client is added to the live subscriber set.
#[derive(Debug, Clone)]
pub struct DaemonSnapshotJournal {
    snapshot: DaemonSnapshot,
    events: VecDeque<SequencedEvent>,
    limits: DaemonSnapshotLimits,
    snapshot_bytes_upper_bound: usize,
    published_events: u64,
    published_event_bytes: u64,
    snapshot_serializations: u64,
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
        snapshot: DaemonSnapshot,
        limits: DaemonSnapshotLimits,
    ) -> Result<Self, DaemonSnapshotError> {
        let limits = limits.validate()?;
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
        let snapshot_bytes = serde_json::to_vec(&snapshot)?.len();
        if snapshot_bytes > limits.snapshot_bytes {
            return Err(DaemonSnapshotError::SnapshotTooLarge {
                actual: snapshot_bytes,
                maximum: limits.snapshot_bytes,
            });
        }
        Ok(Self {
            snapshot,
            events: VecDeque::new(),
            limits,
            snapshot_bytes_upper_bound: snapshot_bytes,
            published_events: 0,
            published_event_bytes: 0,
            snapshot_serializations: 1,
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
        let sequenced = SequencedEvent {
            sequence,
            generation,
            event,
        };
        let event_bytes = serde_json::to_vec(&sequenced)?.len();
        if event_bytes > MAX_FRAME_BYTES {
            return Err(DaemonSnapshotError::EventTooLarge {
                actual: event_bytes,
                maximum: MAX_FRAME_BYTES,
            });
        }
        let conservative_bytes = self.snapshot_bytes_upper_bound.saturating_add(event_bytes);
        let snapshot_bytes_upper_bound = if conservative_bytes <= self.limits.snapshot_bytes
            && matches!(
                &sequenced.event,
                DaemonEvent::Build(_)
                    | DaemonEvent::Log(_)
                    | DaemonEvent::JobChanged(_)
                    | DaemonEvent::Telemetry(_)
            ) {
            // Build, log, job, and telemetry reduction cannot reject a payload
            // after the frame and sequence checks above. Telemetry only advances
            // the snapshot counters; its encoded event also covers their growth.
            // Apply these recurring records in
            // place while the conservative size ledger proves that the
            // resulting snapshot remains bounded; validation-sensitive event
            // variants retain the transactional clone below.
            apply_sequenced_event(&mut self.snapshot, &sequenced)?;
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
            let mut encoded_bytes = serde_json::to_vec(&candidate)?.len();
            self.snapshot_serializations = self.snapshot_serializations.saturating_add(1);
            while encoded_bytes > self.limits.snapshot_bytes && !candidate.recent_logs.is_empty() {
                candidate.recent_logs.remove(0);
                encoded_bytes = serde_json::to_vec(&candidate)?.len();
                self.snapshot_serializations = self.snapshot_serializations.saturating_add(1);
            }
            while encoded_bytes > self.limits.snapshot_bytes && candidate.pty_screens.len() > 1 {
                candidate.pty_screens.remove(0);
                encoded_bytes = serde_json::to_vec(&candidate)?.len();
                self.snapshot_serializations = self.snapshot_serializations.saturating_add(1);
            }
            if encoded_bytes > self.limits.snapshot_bytes {
                return Err(DaemonSnapshotError::SnapshotTooLarge {
                    actual: encoded_bytes,
                    maximum: self.limits.snapshot_bytes,
                });
            }
            self.snapshot = candidate;
            encoded_bytes
        };
        self.snapshot_bytes_upper_bound = snapshot_bytes_upper_bound;
        self.published_events = self.published_events.saturating_add(1);
        self.published_event_bytes = self
            .published_event_bytes
            .saturating_add(u64::try_from(event_bytes).unwrap_or(u64::MAX));
        self.events.push_back(sequenced.clone());
        while self.events.len() > self.limits.retained_events {
            self.events.pop_front();
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
