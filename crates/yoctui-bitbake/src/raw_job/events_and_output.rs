#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RawJobPlannerError {
    #[error("invalid Raw job request: {0}")]
    InvalidRequest(String),
    #[error("interactive Raw request cannot enter the line-oriented job runner")]
    InteractiveRequest,
    #[error("noninteractive Raw request cannot enter the PTY runner")]
    NoninteractiveRequest,
    #[error("Raw stdout and stderr stream identities must be distinct")]
    DuplicateStreamIdentity,
    #[error("Raw job authorization failed: {0}")]
    Authorization(String),
    #[error("Raw job preview digest or reviewed typed intent changed")]
    PreviewMismatch,
    #[error("initialized environment has no authoritative BitBake executable")]
    MissingExecutableAuthority,
    #[error("unsafe Raw build directory {0}: {1:?}")]
    UnsafeBuildDirectory(PathBuf, io::ErrorKind),
    #[error("unsafe Raw executable {0}: {1:?}")]
    UnsafeExecutable(PathBuf, io::ErrorKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawJobRunnerEvent {
    Started,
    Output(RawOutputChunk),
    Completed {
        exit_code: i32,
    },
    Failed {
        exit_code: Option<i32>,
        message: String,
    },
    TimedOut {
        forced: bool,
        exit_code: Option<i32>,
    },
    Cancelled {
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        message: String,
    },
}

#[derive(Debug)]
enum RawJobPipeEvent {
    Output {
        stream: RawOutputStream,
        text: String,
        truncated_bytes: u64,
    },
    Failed {
        stream: RawOutputStream,
        message: String,
    },
}

async fn read_raw_job_output<R>(
    stream: R,
    kind: RawOutputStream,
    sender: tokio::sync::mpsc::Sender<RawJobPipeEvent>,
) where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut retained = Vec::new();
    let mut truncated_bytes = 0_u64;
    loop {
        let buffer = match reader.fill_buf().await {
            Ok(buffer) => buffer,
            Err(error) => {
                let _ = sender
                    .send(RawJobPipeEvent::Failed {
                        stream: kind,
                        message: error.to_string(),
                    })
                    .await;
                return;
            }
        };
        if buffer.is_empty() {
            if (!retained.is_empty() || truncated_bytes > 0)
                && sender
                    .send(RawJobPipeEvent::Output {
                        stream: kind,
                        text: bounded_raw_output_text(&retained, &mut truncated_bytes),
                        truncated_bytes,
                    })
                    .await
                    .is_err()
            {
                return;
            }
            return;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(buffer.len());
        let remaining = yoctui_model::MAX_RAW_OUTPUT_CHUNK_BYTES.saturating_sub(retained.len());
        retained.extend_from_slice(&buffer[..take.min(remaining)]);
        truncated_bytes = truncated_bytes.saturating_add(take.saturating_sub(remaining) as u64);
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(RawJobPipeEvent::Output {
                    stream: kind,
                    text: bounded_raw_output_text(&retained, &mut truncated_bytes),
                    truncated_bytes,
                })
                .await
                .is_err()
            {
                return;
            }
            retained.clear();
            truncated_bytes = 0;
        }
    }
}

fn bounded_raw_output_text(bytes: &[u8], truncated_bytes: &mut u64) -> String {
    let mut text = output_text(bytes);
    if text.len() <= yoctui_model::MAX_RAW_OUTPUT_CHUNK_BYTES {
        return text;
    }
    let boundary = yoctui_utils::utf8_prefix(&text, yoctui_model::MAX_RAW_OUTPUT_CHUNK_BYTES).len();
    *truncated_bytes = truncated_bytes.saturating_add((text.len() - boundary) as u64);
    text.truncate(boundary);
    text
}
