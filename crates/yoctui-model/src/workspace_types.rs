//! Workspace types.
use super::*;

pub const MAX_ACTIVE_TASKS: usize = 4_096;
pub(crate) const MAX_COMPLETED_TASKS: usize = 1_024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticInfo {
    pub category: String,
    pub summary: String,
    #[serde(default)]
    pub event_metadata: Vec<(String, String)>,
    #[serde(default)]
    pub suggestions: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    #[serde(default)]
    pub id: u64,
    pub severity: Severity,
    pub message: String,
    pub recipe: Option<String>,
    pub task: Option<String>,
    pub path: Option<PathBuf>,
    pub timestamp: SystemTime,
    #[serde(default)]
    pub build: Option<String>,
    #[serde(default)]
    pub protected: bool,
    #[serde(default)]
    pub diagnostic: Option<DiagnosticInfo>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Workspace {
    pub build_dir: Option<PathBuf>,
    pub source_dir: Option<PathBuf>,
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub variable_provenance: HashMap<String, String>,
    #[serde(default)]
    pub variable_provenance_chain: HashMap<String, Vec<String>>,
    pub bitbake_version: Option<String>,
    #[serde(default)]
    pub release: Option<String>,
    pub layers: Vec<Layer>,
    pub recipes: Vec<Recipe>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariableIdentity {
    pub name: String,
    pub recipe: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableOperation {
    pub operation: String,
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub value: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDetail {
    pub identity: VariableIdentity,
    pub effective_value: Option<String>,
    pub unexpanded_value: Option<String>,
    pub provenance: Option<String>,
    pub operations: Vec<VariableOperation>,
    pub active_overrides: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigCopyValue {
    Effective,
    Unexpanded,
}
