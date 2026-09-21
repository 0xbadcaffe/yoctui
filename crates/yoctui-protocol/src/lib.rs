//! Versioned, newline-delimited JSON protocol shared with the Python bridge.
pub mod daemon;
#[cfg(unix)]
pub mod daemon_ipc;
#[cfg(unix)]
pub mod daemon_lifecycle;
#[cfg(unix)]
pub mod daemon_persist;
pub mod rootfs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
pub const VERSION: u32 = 1;
pub const MAX_LINE_BYTES: usize = 1024 * 1024;
pub const MAX_RECIPE_CHUNK_BYTES: usize = 512 * 1024;
pub const MAX_RECIPE_INVENTORY_BYTES: usize = 3 * 1024 * 1024;
pub const MAX_RECIPE_INVENTORY_RECORDS: usize = 16384;
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
    Hello {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        compatibility: Option<Box<BridgeCompatibilityData>>,
    },
    InspectWorkspace,
    StartBuild {
        targets: Vec<String>,
        task: Option<String>,
        #[serde(default)]
        force: bool,
    },
    CancelBuild,
    TerminateServer,
    ListRecipes {
        filter: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        chunked: bool,
    },
    ListLayers,
    GetVariable {
        name: String,
        recipe: Option<String>,
    },
    GetDependencies {
        recipe: String,
    },
    GetDependencyGraph {
        recipe: String,
    },
    GetRecipeSources {
        recipe: String,
    },
    GetRecipeMetadata {
        recipe: String,
    },
    GetLayerRelationships,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeCapabilityData {
    pub id: String,
    pub implementation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeCompatibilityData {
    pub generation: u64,
    pub build_directory: String,
    pub capabilities: Vec<BridgeCapabilityData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecipeData {
    pub name: String,
    pub version: Option<String>,
    pub layer: Option<String>,
    #[serde(default)]
    pub preferred_version: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub append_count: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyNodeIdData {
    pub recipe: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyNodeData {
    pub id: DependencyNodeIdData,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyEdgeKindData {
    Build,
    Runtime,
    Task,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyEdgeData {
    pub from: DependencyNodeIdData,
    pub to: DependencyNodeIdData,
    pub kind: DependencyEdgeKindData,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyGraphData {
    pub root: DependencyNodeIdData,
    pub nodes: Vec<DependencyNodeData>,
    pub edges: Vec<DependencyEdgeData>,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecipeWorkspaceStatusData {
    Clean,
    Modified,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecipeBuildStatusData {
    Idle,
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecipeMetadataData {
    pub recipe: String,
    #[serde(default)]
    pub workspace_status: Option<RecipeWorkspaceStatusData>,
    #[serde(default)]
    pub build_status: Option<RecipeBuildStatusData>,
    #[serde(default)]
    pub tasks: Option<Vec<String>>,
    #[serde(default)]
    pub sources: Option<Vec<String>>,
    #[serde(default)]
    pub patches: Option<Vec<String>>,
    #[serde(default)]
    pub packages: Option<Vec<String>>,
    #[serde(default)]
    pub history: Option<Vec<String>>,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariableOperationData {
    pub operation: String,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub line: Option<u32>,
    #[serde(default)]
    pub value: Option<String>,
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
        #[serde(default)]
        compatibility_generation: Option<u64>,
        #[serde(default)]
        capabilities: Vec<String>,
    },
    Workspace {
        data: WorkspaceData,
    },
    Recipes {
        recipes: Vec<RecipeData>,
    },
    RecipesChunk {
        offset: usize,
        total: usize,
        complete: bool,
        recipes: Vec<RecipeData>,
    },
    Layers {
        layers: Vec<LayerData>,
    },
    Variable {
        name: String,
        #[serde(default)]
        recipe: Option<String>,
        value: Option<String>,
        #[serde(default)]
        provenance: Option<String>,
        #[serde(default)]
        unexpanded_value: Option<String>,
        #[serde(default)]
        operations: Vec<VariableOperationData>,
        #[serde(default)]
        active_overrides: Vec<String>,
    },
    Dependencies {
        recipe: String,
        build: Vec<String>,
        runtime: Vec<String>,
    },
    DependencyGraph {
        data: DependencyGraphData,
    },
    RecipeSources {
        recipe: String,
        paths: Vec<String>,
    },
    RecipeMetadata {
        data: RecipeMetadataData,
    },
    LayerRelationships {
        layers: Vec<LayerRelationshipData>,
    },
    BuildStarted,
    TaskStats {
        stats: TaskStatsData,
    },
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
    ServerTerminated,
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
#[path = "tests/lib/mod.rs"]
mod tests;

pub mod build_archive;
