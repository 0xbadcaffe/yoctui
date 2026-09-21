use super::*;

pub struct BridgeBackend {
    pub(crate) child: Child,
    pub(crate) stdin: ChildStdin,
    pub(crate) lines: BufReader<tokio::process::ChildStdout>,
    pub(crate) sequence: u64,
    pub(crate) last_sequence: u64,
    pub(crate) accepted_correlations: VecDeque<String>,
    pub(crate) signature_adapter: SignatureAdapter,
    pub(crate) api_authority: Option<BitBakeApiAuthority>,
    pub(crate) stderr_tail: Arc<Mutex<BridgeStderrTail>>,
    pub(crate) stderr_task: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BridgeProcessPriority {
    #[default]
    Inherited,
    Background,
}

const BACKGROUND_BRIDGE_NICE: i32 = 10;

#[derive(Default)]
pub(crate) struct BridgeStderrTail {
    pub(crate) bytes: VecDeque<u8>,
    pub(crate) truncated: bool,
}

impl BridgeStderrTail {
    pub(crate) fn push(&mut self, bytes: &[u8]) {
        if bytes.len() >= MAX_BRIDGE_STDERR_BYTES {
            self.bytes.clear();
            self.bytes.extend(
                bytes[bytes.len().saturating_sub(MAX_BRIDGE_STDERR_BYTES)..]
                    .iter()
                    .copied(),
            );
            self.truncated = true;
            return;
        }
        let overflow = self
            .bytes
            .len()
            .saturating_add(bytes.len())
            .saturating_sub(MAX_BRIDGE_STDERR_BYTES);
        if overflow > 0 {
            self.bytes.drain(..overflow);
            self.truncated = true;
        }
        self.bytes.extend(bytes.iter().copied());
    }

    pub(crate) fn diagnostic(&self) -> Option<String> {
        if self.bytes.is_empty() {
            return None;
        }
        let bytes = self.bytes.iter().copied().collect::<Vec<_>>();
        let text = String::from_utf8_lossy(&bytes);
        let mut output = String::new();
        if self.truncated {
            output.push_str("[earlier bridge stderr truncated]\n");
        }
        for line in text.lines() {
            let normalized = line.to_ascii_lowercase();
            if [
                "password",
                "passwd",
                "secret",
                "token",
                "credential",
                "api_key",
            ]
            .iter()
            .any(|word| normalized.contains(word))
            {
                output.push_str("[redacted sensitive diagnostic]");
            } else {
                output.extend(line.chars().map(|character| {
                    if character.is_control() && character != '\t' {
                        '�'
                    } else {
                        character
                    }
                }));
            }
            output.push('\n');
        }
        let output = output.trim().to_owned();
        (!output.is_empty()).then_some(output)
    }
}

pub(crate) async fn drain_bridge_stderr<R>(mut stderr: R, tail: Arc<Mutex<BridgeStderrTail>>)
where
    R: AsyncRead + Unpin,
{
    let mut buffer = [0_u8; 4096];
    loop {
        let count = match stderr.read(&mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(count) => count,
        };
        if let Ok(mut tail) = tail.lock() {
            tail.push(&buffer[..count]);
        }
    }
}

pub(crate) const BUNDLED_BRIDGE_SOURCE: &str = include_str!("../../bridge/yoctui_bridge.py");
