/// Make room by evicting retained history, never authoritative workspace,
/// active job/session metadata, build outcome or aggregate progress. Live
/// subscribers still receive the complete event; new clients receive a bounded
/// checkpoint, with explicit dropped-output counters for Raw commands.
fn compact_snapshot(
    snapshot: &mut DaemonSnapshot,
    maximum: usize,
    serializations: &mut u64,
) -> Result<usize, DaemonSnapshotError> {
    let mut bytes = serde_json::to_vec(snapshot)?.len();
    *serializations = serializations.saturating_add(1);
    while bytes > maximum {
        let mut needed = bytes - maximum;
        evict_oldest(&mut snapshot.recent_logs, &mut needed, |_| true)?;
        // Output can be large even for a single completed command. Preserve
        // request/result/cancellation metadata and the newest output tail.
        for execution in &mut snapshot.raw_executions {
            for output in [&mut execution.stdout, &mut execution.stderr] {
                trim_raw_output(output, &mut needed)?;
            }
        }
        // Screen images are reconstructible by the PTY supervisor. Even the
        // final cached screen may be evicted to admit a reliable state update;
        // the terminal session itself and its writer lease remain intact.
        evict_oldest(&mut snapshot.pty_screens, &mut needed, |_| true)?;
        evict_oldest(&mut snapshot.build_events, &mut needed, |event| {
            matches!(event, DaemonBuildEvent::TaskCompleted { .. })
        })?;
        evict_oldest(&mut snapshot.recovery_warnings, &mut needed, |_| true)?;
        evict_oldest(&mut snapshot.jobs, &mut needed, |job| {
            matches!(
                job.lifecycle,
                LifecycleState::Exited
                    | LifecycleState::Cancelled
                    | LifecycleState::Failed
                    | LifecycleState::Lost
            )
        })?;
        let compacted = serde_json::to_vec(snapshot)?.len();
        *serializations = serializations.saturating_add(1);
        if compacted >= bytes {
            return Err(DaemonSnapshotError::SnapshotTooLarge {
                actual: compacted,
                maximum,
            });
        }
        bytes = compacted;
    }
    Ok(bytes)
}

fn trim_raw_output(
    output: &mut RawRetainedOutputData,
    needed: &mut usize,
) -> Result<(), DaemonSnapshotError> {
    let mut count = 0;
    for chunk in &output.chunks {
        if *needed == 0 {
            break;
        }
        *needed = needed.saturating_sub(serde_json::to_vec(chunk)?.len());
        let bytes = chunk.text.len() as u64;
        let lines = raw_protocol_line_count(&chunk.text) as u64;
        output.retained_bytes -= bytes;
        output.retained_lines -= lines;
        output.dropped_bytes = output.dropped_bytes.saturating_add(bytes);
        output.dropped_lines = output.dropped_lines.saturating_add(lines);
        count += 1;
    }
    output.chunks.drain(..count);
    Ok(())
}

// Measure only removed leaves, then encode the whole checkpoint once per
// compaction pass, rather than once per log/chunk/task (quadratic under load).
fn evict_oldest<T: Serialize>(
    items: &mut Vec<T>,
    needed: &mut usize,
    removable: impl Fn(&T) -> bool,
) -> Result<(), DaemonSnapshotError> {
    let mut error = None;
    let mut remaining = items.len();
    items.retain(|item| {
        if *needed == 0 || !removable(item) || error.is_some() {
            return true;
        }
        match serde_json::to_vec(item) {
            Ok(encoded) => {
                *needed = needed.saturating_sub(encoded.len() + usize::from(remaining > 1));
                remaining -= 1;
                false
            }
            Err(failure) => {
                error = Some(failure);
                true
            }
        }
    });
    match error {
        Some(error) => Err(error.into()),
        None => Ok(()),
    }
}
