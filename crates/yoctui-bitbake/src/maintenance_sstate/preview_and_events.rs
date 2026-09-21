fn validate_cleanup_request(
    snapshot: &MaintenanceCapabilitySnapshot,
    request: &SstateCleanupRequest,
) -> Result<(), MaintenanceSstateAdapterError> {
    let cache = canonical_directory(&request.cache_dir)?;
    if snapshot.metadata.sstate_dir.as_ref() != Some(&cache) {
        return Err(MaintenanceSstateAdapterError::PreviewMismatch);
    }
    let mut stamps = request
        .stamps_dirs
        .iter()
        .map(|path| canonical_directory(path))
        .collect::<Result<Vec<_>, _>>()?;
    stamps.sort();
    stamps.dedup();
    if stamps != request.stamps_dirs
        || stamps
            .iter()
            .any(|path| !snapshot.metadata.stamps_dirs.contains(path))
    {
        return Err(MaintenanceSstateAdapterError::PreviewMismatch);
    }
    Ok(())
}

pub fn parse_cleanup_preview(
    request: SstateCleanupRequest,
    lines: &[String],
) -> Result<SstateCleanupPreview, MaintenanceSstateAdapterError> {
    if lines.iter().map(String::len).sum::<usize>() > MAX_PREVIEW_OUTPUT_BYTES {
        return Err(MaintenanceSstateAdapterError::InvalidPreviewOutput(
            "preview output exceeded the byte limit".into(),
        ));
    }
    let mut candidates = Vec::new();
    for line in lines.iter().take(MAX_MAINTENANCE_PATHS + 1) {
        let path = Path::new(line.trim());
        if !path.is_absolute() {
            continue;
        }
        candidates.push(identity_for_candidate(path, &request.cache_dir)?);
    }
    if candidates.len() > MAX_MAINTENANCE_PATHS {
        return Err(MaintenanceSstateAdapterError::InvalidPreviewOutput(
            "preview candidate count exceeded the limit".into(),
        ));
    }
    SstateCleanupPreview::new(request, candidates)
        .map_err(|message| MaintenanceSstateAdapterError::InvalidPreviewOutput(message.into()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceSstateRunnerEvent {
    Started {
        id: MaintenanceSessionId,
    },
    Output {
        id: MaintenanceSessionId,
        stream: MaintenanceOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        id: MaintenanceSessionId,
        exit_code: Option<i32>,
    },
    Failed {
        id: MaintenanceSessionId,
        exit_code: Option<i32>,
    },
    CancellationRequested {
        id: MaintenanceSessionId,
    },
    Cancelled {
        id: MaintenanceSessionId,
        forced: bool,
        exit_code: Option<i32>,
    },
    CancellationRejected {
        id: MaintenanceSessionId,
        message: String,
    },
    TimedOut {
        id: MaintenanceSessionId,
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        id: MaintenanceSessionId,
        message: String,
    },
}

#[derive(Debug)]
enum PipeEvent {
    Output {
        stream: MaintenanceOutputStream,
        line: String,
        truncated: bool,
    },
    Failed {
        stream: MaintenanceOutputStream,
        message: String,
    },
}

async fn read_output<R>(
    stream: R,
    kind: MaintenanceOutputStream,
    sender: tokio::sync::mpsc::Sender<PipeEvent>,
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
                    .send(PipeEvent::Failed {
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
                    .send(PipeEvent::Output {
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
            let remaining = MAX_MAINTENANCE_TEXT_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(PipeEvent::Output {
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

fn output_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\r')
        .to_string()
}

async fn write_process_stdin<W>(stdin: &mut W, payload: &[u8]) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    match stdin.write_all(payload).await {
        Err(error) if error.kind() == ErrorKind::BrokenPipe => Ok(()),
        result => result,
    }
}

async fn spawn_process(process: &mut Command) -> std::io::Result<Child> {
    for attempt in 1..=SPAWN_ATTEMPTS {
        match process.spawn() {
            Ok(child) => return Ok(child),
            Err(error) if attempt < SPAWN_ATTEMPTS && is_transient_spawn_error(&error) => {
                tokio::time::sleep(SPAWN_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the bounded spawn loop always returns")
}
