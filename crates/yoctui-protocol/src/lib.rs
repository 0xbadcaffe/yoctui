//! Versioned, newline-delimited JSON protocol shared with the Python bridge.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
pub const VERSION: u32 = 1;
pub const MAX_LINE_BYTES: usize = 1024 * 1024;
#[derive(Debug, Default)]
pub struct LineFramer {
    pending: Vec<u8>,
}

impl LineFramer {
    /// Adds arbitrary transport bytes and returns only complete newline-delimited frames.
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, ProtocolError> {
        let mut frames = Vec::new();
        for byte in bytes {
            if *byte == b'\n' {
                frames.push(std::mem::take(&mut self.pending));
            } else {
                self.pending.push(*byte);
                if self.pending.len() > MAX_LINE_BYTES {
                    self.pending.clear();
                    return Err(ProtocolError::TooLarge);
                }
            }
        }
        Ok(frames)
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("message exceeds {MAX_LINE_BYTES} byte limit")]
    TooLarge,
    #[error("invalid UTF-8")]
    Utf8,
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported protocol version {0}")]
    Version(u32),
    #[error("non-monotonic sequence {actual}, expected greater than {previous}")]
    Sequence { previous: u64, actual: u64 },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Envelope<T> {
    pub protocol_version: u32,
    pub sequence: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    pub message: T,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Hello,
    InspectWorkspace,
    StartBuild {
        targets: Vec<String>,
        task: Option<String>,
    },
    CancelBuild,
    ListRecipes {
        filter: Option<String>,
    },
    ListLayers,
    GetVariable {
        name: String,
        recipe: Option<String>,
    },
    GetDependencies {
        recipe: String,
    },
    GetRecipeSources {
        recipe: String,
    },
    GetLayerRelationships,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecipeData {
    pub name: String,
    pub version: Option<String>,
    pub layer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayerData {
    pub name: String,
    pub path: String,
    pub priority: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayerRelationshipData {
    pub name: String,
    pub priority: Option<i32>,
    pub compatible: Vec<String>,
    pub depends: Vec<String>,
    pub overlays: Vec<String>,
    pub appends: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskStatsData {
    pub completed: usize,
    pub total: usize,
    pub active: usize,
    pub failed: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceData {
    pub build_dir: Option<String>,
    pub source_dir: Option<String>,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub variable_provenance: HashMap<String, String>,
    #[serde(default)]
    pub variable_provenance_chain: HashMap<String, Vec<String>>,
    pub bitbake_version: Option<String>,
    #[serde(default)]
    pub release: Option<String>,
    #[serde(default)]
    pub layers: Vec<LayerData>,
    #[serde(default)]
    pub recipes: Vec<RecipeData>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    HelloAck {
        bitbake_version: Option<String>,
    },
    Workspace {
        data: WorkspaceData,
    },
    Recipes {
        recipes: Vec<RecipeData>,
    },
    Layers {
        layers: Vec<LayerData>,
    },
    Variable {
        name: String,
        value: Option<String>,
        #[serde(default)]
        provenance: Option<String>,
    },
    Dependencies {
        recipe: String,
        build: Vec<String>,
        runtime: Vec<String>,
    },
    RecipeSources {
        recipe: String,
        paths: Vec<String>,
    },
    LayerRelationships {
        layers: Vec<LayerRelationshipData>,
    },
    BuildStarted,
    ParseProgress {
        current: Option<u64>,
        total: Option<u64>,
    },
    TaskQueued {
        recipe: String,
        task: String,
        #[serde(default)]
        worker: Option<String>,
        #[serde(default)]
        stats: Option<TaskStatsData>,
    },
    TaskStarted {
        recipe: String,
        task: String,
        pid: Option<u32>,
        #[serde(default)]
        worker: Option<String>,
        #[serde(default)]
        log_path: Option<String>,
        #[serde(default)]
        stats: Option<TaskStatsData>,
    },
    TaskProgress {
        recipe: String,
        task: String,
        progress: Option<u8>,
    },
    TaskCompleted {
        recipe: String,
        task: String,
        success: bool,
    },
    Log {
        level: String,
        message: String,
        recipe: Option<String>,
        task: Option<String>,
        path: Option<String>,
    },
    Warning {
        message: String,
    },
    Error {
        message: String,
    },
    BuildCompleted {
        success: bool,
        #[serde(default)]
        exit_code: Option<i32>,
    },
    CommandFailed {
        code: String,
        message: String,
    },
    ProtocolError {
        code: String,
        message: String,
    },
    BridgeShutdown,
    #[serde(other)]
    Unknown,
}
pub fn decode_line<T: for<'de> Deserialize<'de>>(
    line: &[u8],
    previous: Option<u64>,
) -> Result<Envelope<T>, ProtocolError> {
    if line.len() > MAX_LINE_BYTES {
        return Err(ProtocolError::TooLarge);
    }
    let text = std::str::from_utf8(line).map_err(|_| ProtocolError::Utf8)?;
    let e: Envelope<T> = serde_json::from_str(text.trim_end_matches('\n'))?;
    if e.protocol_version != VERSION {
        return Err(ProtocolError::Version(e.protocol_version));
    }
    if let Some(p) = previous.filter(|p| e.sequence <= *p) {
        return Err(ProtocolError::Sequence {
            previous: p,
            actual: e.sequence,
        });
    }
    Ok(e)
}
pub fn encode_line<T: Serialize>(e: &Envelope<T>) -> Result<Vec<u8>, ProtocolError> {
    let mut v = serde_json::to_vec(e)?;
    v.push(b'\n');
    Ok(v)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    #[test]
    fn round_trip() {
        let e = Envelope {
            protocol_version: 1,
            sequence: 1,
            correlation_id: Some("x".into()),
            message: Command::Hello,
        };
        assert_eq!(
            decode_line::<Command>(&encode_line(&e).unwrap(), None).unwrap(),
            e
        )
    }
    #[test]
    fn rejects_sequence() {
        let v = br#"{"protocol_version":1,"sequence":2,"message":{"type":"hello"}}"#;
        assert!(matches!(
            decode_line::<Command>(v, Some(2)),
            Err(ProtocolError::Sequence { .. })
        ))
    }
    #[test]
    fn unknown_event_is_safe() {
        let v = br#"{"protocol_version":1,"sequence":2,"message":{"type":"future_event"}}"#;
        assert_eq!(
            decode_line::<Event>(v, None).unwrap().message,
            Event::Unknown
        )
    }
    #[test]
    fn typed_event_workspace_round_trips_without_untyped_json() {
        let event = Event::Workspace {
            data: WorkspaceData {
                build_dir: Some("/build".into()),
                source_dir: Some("/poky".into()),
                variables: HashMap::from([("MACHINE".into(), "qemux86-64".into())]),
                variable_provenance: HashMap::new(),
                variable_provenance_chain: HashMap::new(),
                bitbake_version: Some("2.19.0".into()),
                release: Some("6.0".into()),
                layers: vec![LayerData {
                    name: "core".into(),
                    path: "/poky/meta".into(),
                    priority: Some(5),
                }],
                recipes: vec![RecipeData {
                    name: "base-files".into(),
                    version: Some("3.0".into()),
                    layer: Some("core".into()),
                }],
            },
        };
        let envelope = Envelope {
            protocol_version: VERSION,
            sequence: 4,
            correlation_id: None,
            message: event.clone(),
        };
        assert_eq!(
            decode_line::<Event>(&encode_line(&envelope).unwrap(), None)
                .unwrap()
                .message,
            event
        );
    }

    #[test]
    fn frames_partial_lines_without_losing_data() {
        let mut framer = LineFramer::default();
        assert!(framer.push(b"one\ntw").unwrap().as_slice() == [b"one".to_vec()]);
        assert_eq!(framer.pending_len(), 2);
        assert_eq!(framer.push(b"o\n").unwrap(), vec![b"two".to_vec()]);
    }

    #[test]
    fn oversized_partial_line_is_rejected_and_cleared() {
        let mut framer = LineFramer::default();
        assert!(matches!(
            framer.push(&vec![b'x'; MAX_LINE_BYTES + 1]),
            Err(ProtocolError::TooLarge)
        ));
        assert_eq!(framer.pending_len(), 0);
    }

    proptest! {
        #[test]
        fn framing_is_independent_of_chunk_boundaries(parts in proptest::collection::vec("[a-z]{0,12}", 0..30), chunk_sizes in proptest::collection::vec(1usize..16, 1..30)) {
            let source = parts.iter().map(|part| format!("{part}\n")).collect::<String>().into_bytes();
            let mut framer = LineFramer::default();
            let mut frames = Vec::new();
            let mut offset = 0;
            for size in chunk_sizes {
                if offset == source.len() { break; }
                let end = (offset + size).min(source.len());
                frames.extend(framer.push(&source[offset..end]).unwrap());
                offset = end;
            }
            if offset < source.len() { frames.extend(framer.push(&source[offset..]).unwrap()); }
            prop_assert_eq!(frames, parts.into_iter().map(String::into_bytes).collect::<Vec<_>>());
            prop_assert_eq!(framer.pending_len(), 0);
        }
    }
}
