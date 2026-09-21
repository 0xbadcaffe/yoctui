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

#[cfg(test)]
mod daemon_test_snapshot_tests {
    use super::*;
    #[test]
    fn daemon_test_snapshot_is_bounded_and_round_trips() {
        let snapshot = DaemonTestResultSnapshot {
            generation: 4,
            records: (0..(MAX_TEST_RESULT_RECORDS + 2))
                .map(|index| DaemonTestResultRecord {
                    identity: index.to_string(),
                    outcome: "pass".into(),
                    duration_ms: None,
                    log_path: None,
                })
                .collect(),
            limitations: (0..(MAX_TEST_RESULT_LIMITATIONS + 2))
                .map(|index| index.to_string())
                .collect(),
            complete: true,
        }
        .bounded();
        assert_eq!(snapshot.records.len(), MAX_TEST_RESULT_RECORDS);
        assert_eq!(snapshot.limitations.len(), MAX_TEST_RESULT_LIMITATIONS);
        let encoded = serde_json::to_vec(&snapshot).unwrap();
        let decoded: DaemonTestResultSnapshot = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, snapshot);
    }

    #[test]
    fn daemon_test_compare_diff_is_bounded_and_round_trips() {
        let diff = DaemonTestComparisonDiff {
            generation: 2,
            baseline: "a".into(),
            candidate: "b".into(),
            transitions: Vec::new(),
            limitations: vec!["limited".into()],
        }
        .bounded();
        let bytes = serde_json::to_vec(&diff).unwrap();
        assert_eq!(
            serde_json::from_slice::<DaemonTestComparisonDiff>(&bytes).unwrap(),
            diff
        );
    }

    #[test]
    fn daemon_qa_snapshot_is_bounded() {
        let snapshot = DaemonQaSnapshot {
            generation: 1,
            capability: "available".into(),
            task_bindings: (0..MAX_QA_RECORDS + 1).map(|i| i.to_string()).collect(),
            reports: (0..MAX_QA_RECORDS + 1).map(|i| i.to_string()).collect(),
            limitations: Vec::new(),
        }
        .bounded();
        assert_eq!(snapshot.task_bindings.len(), MAX_QA_RECORDS);
        assert_eq!(snapshot.reports.len(), MAX_QA_RECORDS);
    }

    #[test]
    fn daemon_qa_input_is_bounded() {
        let input = DaemonQaCapabilityInput {
            generation: 1,
            build_directory: "/build".into(),
            source_directory: None,
            layer_directories: (0..MAX_QA_RECORDS + 1).map(|i| i.to_string()).collect(),
            recipe_names: Vec::new(),
            report_roots: Vec::new(),
            selected_recipe_name: "recipe".into(),
            selected_recipe_file: "/build/recipe.bb".into(),
        }
        .bounded();
        assert_eq!(input.layer_directories.len(), MAX_QA_RECORDS);
    }
}

impl DaemonTestResultSnapshot {
    pub fn bounded(mut self) -> Self {
        self.records.truncate(MAX_TEST_RESULT_RECORDS);
        self.limitations.truncate(MAX_TEST_RESULT_LIMITATIONS);
        self
    }
}
