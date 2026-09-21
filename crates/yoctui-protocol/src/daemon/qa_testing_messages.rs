pub const MAX_TEST_RESULT_RECORDS: usize = 4096;
pub const MAX_TEST_RESULT_LIMITATIONS: usize = 256;
pub const MAX_QA_RECORDS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestResultRecord {
    pub identity: String,
    pub outcome: String,
    pub duration_ms: Option<u64>,
    pub log_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonQaSnapshot {
    pub generation: u64,
    pub capability: String,
    pub task_bindings: Vec<String>,
    pub reports: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonQaCapabilityInput {
    pub generation: u64,
    pub build_directory: String,
    pub source_directory: Option<String>,
    pub layer_directories: Vec<String>,
    pub recipe_names: Vec<String>,
    pub report_roots: Vec<String>,
    pub selected_recipe_name: String,
    pub selected_recipe_file: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonQaCapabilityRequest {
    pub request_id: RequestId,
    pub input: DaemonQaCapabilityInput,
}

impl DaemonQaCapabilityInput {
    pub fn bounded(mut self) -> Self {
        self.layer_directories.truncate(MAX_QA_RECORDS);
        self.recipe_names.truncate(MAX_QA_RECORDS);
        self.report_roots.truncate(MAX_QA_RECORDS);
        self
    }
}

impl DaemonQaSnapshot {
    pub fn bounded(mut self) -> Self {
        self.task_bindings.truncate(MAX_QA_RECORDS);
        self.reports.truncate(MAX_QA_RECORDS);
        self.limitations.truncate(MAX_TEST_RESULT_LIMITATIONS);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestResultSnapshot {
    pub generation: u64,
    pub records: Vec<DaemonTestResultRecord>,
    pub limitations: Vec<String>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestComparisonDiff {
    pub generation: u64,
    pub baseline: String,
    pub candidate: String,
    pub transitions: Vec<DaemonTestComparisonTransition>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonTestResultToolCapability {
    NotInspected,
    Missing,
    Available { executable: String },
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonTestComparisonTransition {
    pub identity: String,
    pub baseline: Option<String>,
    pub candidate: Option<String>,
    pub category: String,
}

impl DaemonTestComparisonDiff {
    pub fn bounded(mut self) -> Self {
        self.transitions.truncate(MAX_TEST_RESULT_RECORDS);
        self.limitations.truncate(MAX_TEST_RESULT_LIMITATIONS);
        self
    }
}



impl DaemonTestResultSnapshot {
    pub fn bounded(mut self) -> Self {
        self.records.truncate(MAX_TEST_RESULT_RECORDS);
        self.limitations.truncate(MAX_TEST_RESULT_LIMITATIONS);
        self
    }
}
