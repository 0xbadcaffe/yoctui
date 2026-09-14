//! Process output.
use super::*;

pub(crate) const MAX_PROCESS_LINE_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_BRIDGE_STDERR_BYTES: usize = 16 * 1024;
pub(crate) const MAX_DEPENDENCY_GRAPH_FILE_BYTES: u64 = 8 * 1024 * 1024;
pub(crate) const MAX_DEPENDENCY_NODES: usize = 2_000;
pub(crate) const MAX_DEPENDENCY_EDGES: usize = 4_000;
pub(crate) const DEPENDENCY_GRAPH_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) async fn read_output<R>(stream: R, sender: tokio::sync::mpsc::Sender<LogEntry>)
where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut bytes = Vec::new();
    let mut discarding = false;
    while let Ok(buffer) = reader.fill_buf().await {
        if buffer.is_empty() {
            if !bytes.is_empty()
                && !discarding
                && sender
                    .send(classify_output(output_text(&bytes)))
                    .await
                    .is_err()
            {
                break;
            }
            break;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(buffer.len());
        if !discarding {
            if bytes.len() + take > MAX_PROCESS_LINE_BYTES {
                let mut message = output_text(&bytes);
                message.push_str(" [line truncated]");
                if sender.send(classify_output(message)).await.is_err() {
                    break;
                }
                bytes.clear();
                discarding = true;
            } else {
                bytes.extend_from_slice(&buffer[..take]);
            }
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if !discarding
                && sender
                    .send(classify_output(output_text(&bytes)))
                    .await
                    .is_err()
            {
                break;
            }
            bytes.clear();
            discarding = false;
        }
    }
}

pub fn output_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches(['\r', '\n'])
        .into()
}
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("process: {0}")]
    Process(#[from] std::io::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("bridge: {0}")]
    Bridge(String),
    #[error("signature: {0}")]
    Signature(#[from] SignatureAdapterError),
    #[error("compatibility API: {0}")]
    CompatibilityApi(#[from] BitBakeApiCompatibilityError),
    #[error("backend is not running")]
    NotRunning,
}
