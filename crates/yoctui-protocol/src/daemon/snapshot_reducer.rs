pub fn apply_sequenced_event(
    snapshot: &mut DaemonSnapshot,
    sequenced: &SequencedEvent,
) -> Result<(), DaemonSnapshotError> {
    let expected_sequence = snapshot
        .sequence
        .checked_add(1)
        .ok_or(DaemonSnapshotError::SequenceExhausted)?;
    let expected_generation = snapshot
        .generation
        .checked_add(1)
        .ok_or(DaemonSnapshotError::GenerationExhausted)?;
    if sequenced.sequence != expected_sequence || sequenced.generation != expected_generation {
        return Err(DaemonSnapshotError::EventGap {
            expected_sequence,
            actual_sequence: sequenced.sequence,
            expected_generation,
            actual_generation: sequenced.generation,
        });
    }
    match &sequenced.event {
        DaemonEvent::BitBakeChanged(bitbake) => snapshot.bitbake = bitbake.clone(),
        DaemonEvent::CompatibilityChanged(compatibility) => {
            compatibility.validate()?;
            if snapshot
                .compatibility
                .as_ref()
                .is_some_and(|current| current.generation >= compatibility.generation)
            {
                return Err(DaemonSnapshotError::StaleCompatibilityGeneration {
                    current: snapshot
                        .compatibility
                        .as_ref()
                        .map(|current| current.generation)
                        .unwrap_or(0),
                    received: compatibility.generation,
                });
            }
            snapshot.compatibility = Some((**compatibility).clone());
        }
        DaemonEvent::JobChanged(job) => {
            if job.kind == JobKind::BitBakeBuild
                && job.lifecycle == LifecycleState::Running
                && matches!(
                    snapshot.build_events.iter().rev().find(|event| matches!(
                        event,
                        DaemonBuildEvent::Started { .. }
                            | DaemonBuildEvent::Completed { .. }
                            | DaemonBuildEvent::Reset { .. }
                            | DaemonBuildEvent::Disconnected
                    )),
                    Some(DaemonBuildEvent::Started { .. })
                )
                && let Some(progress) = &mut snapshot.build_progress
                && let Some(completed) = job
                    .progress_current
                    .and_then(|value| usize::try_from(value).ok())
            {
                progress.completed = progress.completed.max(completed);
                progress.total = job
                    .progress_total
                    .and_then(|value| usize::try_from(value).ok())
                    .filter(|value| *value > 0);
            }
            replace_by(&mut snapshot.jobs, job.clone(), |item| item.id);
        }
        DaemonEvent::JobRemoved { job_id } => snapshot.jobs.retain(|job| job.id != *job_id),
        DaemonEvent::RawExecutionChanged(execution) => {
            execution.validate()?;
            let request_id = execution.request.request_id.clone();
            if let Some(current) = snapshot
                .raw_executions
                .iter()
                .find(|current| current.request.request_id == request_id)
                && (execution.sequence <= current.sequence
                    || execution.generation <= current.generation)
            {
                return Err(DaemonSnapshotError::StaleRawExecution {
                    request_id,
                    current_sequence: current.sequence,
                    received_sequence: execution.sequence,
                });
            }
            replace_by(
                &mut snapshot.raw_executions,
                (**execution).clone(),
                |item| item.request.request_id.clone(),
            );
            if snapshot.raw_executions.len() > MAX_RAW_EXECUTION_REQUESTS {
                return Err(DaemonSnapshotError::TooManyRawExecutions);
            }
            if execution.phase.is_terminal() {
                remember_raw_history(snapshot, RawHistoryRecordData::from_terminal(execution)?);
                validate_raw_history_records(&snapshot.raw_history)?;
            }
        }
        DaemonEvent::RawExecutionRemoved { request_id } => {
            validate_raw_identity(request_id, "raw-request:", "request")?;
            snapshot
                .raw_executions
                .retain(|execution| execution.request.request_id != *request_id);
        }
        DaemonEvent::PtyChanged(pty) => {
            replace_by(&mut snapshot.pty_sessions, pty.clone(), |item| item.id);
        }
        DaemonEvent::PtyRemoved { session_id } => {
            snapshot.pty_sessions.retain(|item| item.id != *session_id);
            snapshot
                .pty_screens
                .retain(|item| item.session_id != *session_id);
        }
        DaemonEvent::PtyScreen(screen) => {
            validate_pty_screen(screen)?;
            snapshot
                .pty_screens
                .retain(|item| item.session_id != screen.session_id);
            snapshot.pty_screens.push(screen.clone());
        }
        DaemonEvent::PtyOutput { .. }
        | DaemonEvent::DevtoolStatusChanged(_)
        | DaemonEvent::TestResults(_)
        | DaemonEvent::TestComparison(_)
        | DaemonEvent::TestResultTool(_)
        | DaemonEvent::QaSnapshot(_)
        | DaemonEvent::QaCapability(_)
        | DaemonEvent::SecuritySnapshot(_)
        | DaemonEvent::MaintenanceSnapshot(_)
        | DaemonEvent::Telemetry(_)
        | DaemonEvent::Unknown => {}
        DaemonEvent::ClientChanged(client) => {
            replace_by(&mut snapshot.clients, client.clone(), |item| item.id);
        }
        DaemonEvent::ClientRemoved { client_id } => {
            snapshot.clients.retain(|client| client.id != *client_id);
        }
        DaemonEvent::RecoveryWarning { message } => {
            snapshot.recovery_warnings.push(message.clone());
        }
        DaemonEvent::Log(record) => snapshot.recent_logs.push(record.clone()),
        DaemonEvent::Build(event) => apply_build_event(snapshot, event.clone()),
    }
    snapshot.sequence = sequenced.sequence;
    snapshot.generation = sequenced.generation;
    Ok(())
}

fn preserve_build_timing(snapshot: &DaemonSnapshot, event: &mut DaemonEvent) {
    let DaemonEvent::Build(event) = event else {
        return;
    };
    match event {
        DaemonBuildEvent::Started { started_unix_ms } => {
            for previous in snapshot.build_events.iter().rev() {
                match previous {
                    DaemonBuildEvent::Reset { .. } => break,
                    DaemonBuildEvent::Started {
                        started_unix_ms: first,
                    } => {
                        *started_unix_ms = *first;
                        break;
                    }
                    _ => {}
                }
            }
        }
        DaemonBuildEvent::TaskCompleted {
            recipe,
            task,
            started_unix_ms,
            finished_unix_ms,
            ..
        } => {
            // Complete records retain their observed start before row compaction
            // removes the matching TaskStarted. Repeated terminal records must
            // retain the first end; a new queue/start is a new observation.
            for previous in snapshot.build_events.iter().rev() {
                match previous {
                    DaemonBuildEvent::TaskStarted {
                        recipe: old_recipe,
                        task: old_task,
                        started_unix_ms: start,
                        ..
                    } if old_recipe == recipe && old_task == task => {
                        if started_unix_ms.is_none() {
                            *started_unix_ms = *start;
                        }
                        break;
                    }
                    DaemonBuildEvent::TaskQueued {
                        recipe: old_recipe,
                        task: old_task,
                        ..
                    } if old_recipe == recipe && old_task == task => break,
                    DaemonBuildEvent::TaskCompleted {
                        recipe: old_recipe,
                        task: old_task,
                        started_unix_ms: first_start,
                        finished_unix_ms: first_end,
                        ..
                    } if old_recipe == recipe && old_task == task => {
                        *started_unix_ms = *first_start;
                        *finished_unix_ms = *first_end;
                        break;
                    }
                    _ => {}
                }
            }
        }
        DaemonBuildEvent::Completed {
            finished_unix_ms, ..
        } => {
            for previous in snapshot.build_events.iter().rev() {
                match previous {
                    DaemonBuildEvent::Reset { .. } | DaemonBuildEvent::Started { .. } => break,
                    DaemonBuildEvent::Completed {
                        finished_unix_ms: first,
                        ..
                    } => {
                        *finished_unix_ms = *first;
                        break;
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn apply_build_event(snapshot: &mut DaemonSnapshot, event: DaemonBuildEvent) {
    update_build_progress(snapshot, &event);
    if matches!(event, DaemonBuildEvent::Reset { .. }) {
        snapshot.build_events.clear();
    }

    match &event {
        DaemonBuildEvent::SstateSummary { .. } => snapshot
            .build_events
            .retain(|item| !matches!(item, DaemonBuildEvent::SstateSummary { .. })),
        DaemonBuildEvent::Workspace { data } => {
            if let (Some(source), Some(build)) = (&data.source_dir, &data.build_dir) {
                snapshot.workspace = Some(WorkspaceIdentity {
                    canonical_source: source.clone(),
                    canonical_build: build.clone(),
                    identity_hash: stable_workspace_hash(source, build),
                });
            }
            snapshot
                .build_events
                .retain(|item| !matches!(item, DaemonBuildEvent::Workspace { .. }));
        }
        DaemonBuildEvent::ParseProgress { .. } => snapshot
            .build_events
            .retain(|item| !matches!(item, DaemonBuildEvent::ParseProgress { .. })),
        DaemonBuildEvent::TaskQueued { recipe, task, .. }
        | DaemonBuildEvent::TaskStarted { recipe, task, .. } => snapshot.build_events.retain(
            |item| {
                !matches!(item,
                    DaemonBuildEvent::TaskQueued { recipe: old_recipe, task: old_task, .. }
                    | DaemonBuildEvent::TaskStarted { recipe: old_recipe, task: old_task, .. }
                    | DaemonBuildEvent::TaskProgress { recipe: old_recipe, task: old_task, .. }
                    if old_recipe == recipe && old_task == task)
            },
        ),
        DaemonBuildEvent::TaskProgress { recipe, task, .. } => snapshot.build_events.retain(
            |item| {
                !matches!(item, DaemonBuildEvent::TaskProgress { recipe: old_recipe, task: old_task, .. } if old_recipe == recipe && old_task == task)
            },
        ),
        DaemonBuildEvent::TaskCompleted { recipe, task, .. } => snapshot.build_events.retain(
            |item| {
                !matches!(item,
                    DaemonBuildEvent::TaskQueued { recipe: old_recipe, task: old_task, .. }
                    | DaemonBuildEvent::TaskStarted { recipe: old_recipe, task: old_task, .. }
                    | DaemonBuildEvent::TaskProgress { recipe: old_recipe, task: old_task, .. }
                    if old_recipe == recipe && old_task == task)
            },
        ),
        _ => {}
    }
    snapshot.build_events.push(event);
    while snapshot.build_events.len() > MAX_DAEMON_BUILD_EVENTS {
        let removable = snapshot
            .build_events
            .iter()
            .position(|item| matches!(item, DaemonBuildEvent::TaskCompleted { .. }));
        let removable = removable.or_else(|| {
            snapshot.build_events.iter().position(|item| {
                !matches!(
                    item,
                    DaemonBuildEvent::Reset { .. }
                        | DaemonBuildEvent::Workspace { .. }
                        | DaemonBuildEvent::Started { .. }
                )
            })
        });
        snapshot.build_events.remove(removable.unwrap_or(0));
    }
}

fn update_build_progress(snapshot: &mut DaemonSnapshot, event: &DaemonBuildEvent) {
    match event {
        DaemonBuildEvent::SstateSummary { summary } => {
            if summary.valid() {
                snapshot
                    .build_progress
                    .get_or_insert_with(Default::default)
                    .cache
                    .summary = Some(*summary);
            }
        }
        DaemonBuildEvent::Reset { .. } => {
            snapshot.build_progress = Some(DaemonBuildProgress::default());
        }
        DaemonBuildEvent::Started { .. } => {
            snapshot.build_progress.get_or_insert_with(Default::default);
        }
        DaemonBuildEvent::TaskQueued {
            stats: Some(stats), ..
        }
        | DaemonBuildEvent::TaskStarted {
            stats: Some(stats), ..
        } => {
            let progress = snapshot.build_progress.get_or_insert_with(Default::default);
            progress.completed = progress.completed.max(stats.completed);
            progress.total = (stats.total > 0).then_some(stats.total);
        }
        DaemonBuildEvent::TaskCompleted {
            recipe,
            task,
            success,
            ..
        } => {
            // Inspect identity before compaction removes the active task. A
            // repeated completion is inert, but a newly started same-ID task
            // may complete again. Never count a legacy snapshot's partial
            // replay as a complete aggregate without a start/stat checkpoint.
            let active = snapshot.build_events.iter().any(|event| {
                matches!(event,
                DaemonBuildEvent::TaskQueued { recipe: old_recipe, task: old_task, .. }
                | DaemonBuildEvent::TaskStarted { recipe: old_recipe, task: old_task, .. }
                if old_recipe == recipe && old_task == task)
            });
            let completed = snapshot.build_events.iter().any(|event| {
                matches!(event,
                DaemonBuildEvent::TaskCompleted { recipe: old_recipe, task: old_task, .. }
                if old_recipe == recipe && old_task == task)
            });
            if (active || !completed)
                && let Some(progress) = &mut snapshot.build_progress
            {
                progress.completed = progress.completed.saturating_add(1);
                progress.cache.record_outcome(task, *success);
            }
        }
        DaemonBuildEvent::Completed { success: true, .. } => {
            if let Some(progress) = &mut snapshot.build_progress
                && let Some(total) = progress.total.filter(|total| *total > 0)
            {
                progress.completed = total;
            }
        }
        _ => {}
    }
}

fn stable_workspace_hash(source: &str, build: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in source.bytes().chain([0]).chain(build.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn replace_by<T, K: PartialEq>(items: &mut Vec<T>, replacement: T, key: impl Fn(&T) -> K) {
    let replacement_key = key(&replacement);
    if let Some(index) = items.iter().position(|item| key(item) == replacement_key) {
        items[index] = replacement;
    } else {
        items.push(replacement);
    }
}

fn validate_pty_screen(screen: &PtyScreenSnapshot) -> Result<(), DaemonSnapshotError> {
    let dimensions = screen.dimensions;
    let capacity = usize::from(dimensions.columns) * usize::from(dimensions.rows);
    let valid_dimensions = dimensions.columns > 0
        && dimensions.columns <= MAX_TERMINAL_COLUMNS
        && dimensions.rows > 0
        && dimensions.rows <= MAX_TERMINAL_ROWS
        && capacity <= MAX_TERMINAL_CELLS;
    let valid_cursor =
        screen.cursor_column < dimensions.columns && screen.cursor_row < dimensions.rows;
    let valid_cells = screen.cells.len() <= capacity
        && screen.cells.iter().all(|cell| {
            (cell.index as usize) < capacity
                && cell.contents.len() <= MAX_TERMINAL_CELL_BYTES
                && !cell.contents.chars().any(char::is_control)
        })
        && screen
            .cells
            .windows(2)
            .all(|pair| pair[0].index < pair[1].index);
    if !valid_dimensions
        || !valid_cursor
        || !valid_cells
        || screen.scrollback_offset > screen.scrollback_lines
        || screen.scrollback_lines as usize > MAX_TERMINAL_SCROLLBACK_LINES
    {
        return Err(DaemonSnapshotError::InvalidPtyScreen(screen.session_id));
    }
    Ok(())
}
