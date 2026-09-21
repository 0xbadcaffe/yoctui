#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResultImportRequest {
    pub generation: u64,
    pub roots: Vec<PathBuf>,
}

impl TestResultImportRequest {
    pub fn new(generation: u64, mut roots: Vec<PathBuf>) -> Result<Self, &'static str> {
        roots.sort();
        roots.dedup();
        if generation == 0
            || roots.is_empty()
            || roots.len() > MAX_TEST_RESULT_PATHS
            || roots.iter().any(|path| !absolute_normal_path(path))
        {
            return Err("test result import request is invalid");
        }
        Ok(Self { generation, roots })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TestResultInventoryState {
    #[default]
    NotLoaded,
    Loading {
        request: TestResultImportRequest,
    },
    AvailableEmpty {
        request: TestResultImportRequest,
    },
    Available {
        request: TestResultImportRequest,
        records: Vec<TestResultRecord>,
    },
    Partial {
        request: TestResultImportRequest,
        records: Vec<TestResultRecord>,
        limitations: Vec<String>,
    },
    Failed {
        request: TestResultImportRequest,
        message: String,
    },
    Cancelled {
        request: TestResultImportRequest,
    },
    TimedOut {
        request: TestResultImportRequest,
    },
    Lost {
        request: TestResultImportRequest,
        message: String,
    },
}

impl TestResultInventoryState {
    pub fn request(&self) -> Option<&TestResultImportRequest> {
        match self {
            Self::NotLoaded => None,
            Self::Loading { request }
            | Self::AvailableEmpty { request }
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Failed { request, .. }
            | Self::Cancelled { request }
            | Self::TimedOut { request }
            | Self::Lost { request, .. } => Some(request),
        }
    }

    pub fn records(&self) -> &[TestResultRecord] {
        match self {
            Self::Available { records, .. } | Self::Partial { records, .. } => records,
            _ => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ResultToolCapability {
    #[default]
    NotInspected,
    Missing,
    Available(PathBuf),
    Failed(String),
}

impl ResultToolCapability {
    pub fn executable(&self) -> Result<PathBuf, &'static str> {
        match self {
            Self::Available(path) if absolute_normal_path(path) => Ok(path.clone()),
            Self::Available(_) => Err("resulttool executable identity is invalid"),
            Self::NotInspected => Err("resulttool has not been inspected"),
            Self::Missing => Err("resulttool is missing"),
            Self::Failed(_) => Err("resulttool inspection failed"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestWorkspaceView {
    Launches,
    Results,
    Comparison,
}

impl TestWorkspaceView {
    pub fn next(self) -> Self {
        match self {
            Self::Launches => Self::Results,
            Self::Results => Self::Comparison,
            Self::Comparison => Self::Launches,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TestResultImportDialog {
    pub input: String,
    pub validation_error: Option<String>,
}

impl TestResultImportDialog {
    pub fn append(&mut self, character: char) {
        if !character.is_control() && self.input.len() + character.len_utf8() <= MAX_TEST_TEXT_BYTES
        {
            self.input.push(character);
            self.validation_error = None;
        }
    }

    pub fn backspace(&mut self) {
        self.input.pop();
        self.validation_error = None;
    }

    pub fn root(&self) -> Result<PathBuf, &'static str> {
        let path = PathBuf::from(&self.input);
        absolute_normal_path(&path)
            .then_some(path)
            .ok_or("result import path must be normalized and absolute")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestComparisonCategory {
    Regression,
    NewFailure,
    NewPass,
    Removed,
    UnchangedOther,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCaseTransition {
    pub identity: TestCaseIdentity,
    pub baseline: Option<TestCaseOutcome>,
    pub candidate: Option<TestCaseOutcome>,
    pub category: TestComparisonCategory,
    pub baseline_log: Option<PathBuf>,
    pub candidate_log: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestComparison {
    pub baseline: TestResultIdentity,
    pub candidate: TestResultIdentity,
    pub transitions: Vec<TestCaseTransition>,
}

impl TestComparison {
    pub fn between(
        baseline: &TestResultRecord,
        candidate: &TestResultRecord,
    ) -> Result<Self, &'static str> {
        if baseline.identity == candidate.identity {
            return Err("test comparison requires distinct exact results");
        }
        let baseline_cases = baseline
            .suites
            .iter()
            .flat_map(|suite| &suite.cases)
            .map(|case| (case.identity.clone(), case))
            .collect::<BTreeMap<_, _>>();
        let candidate_cases = candidate
            .suites
            .iter()
            .flat_map(|suite| &suite.cases)
            .map(|case| (case.identity.clone(), case))
            .collect::<BTreeMap<_, _>>();
        let identities = baseline_cases
            .keys()
            .chain(candidate_cases.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let transitions = identities
            .into_iter()
            .map(|identity| {
                let baseline = baseline_cases.get(&identity).copied();
                let candidate = candidate_cases.get(&identity).copied();
                let category = classify_test_transition(
                    baseline.map(|case| case.outcome),
                    candidate.map(|case| case.outcome),
                );
                TestCaseTransition {
                    identity,
                    baseline: baseline.map(|case| case.outcome),
                    candidate: candidate.map(|case| case.outcome),
                    category,
                    baseline_log: baseline.and_then(|case| case.log_path.clone()),
                    candidate_log: candidate.and_then(|case| case.log_path.clone()),
                }
            })
            .collect();
        Ok(Self {
            baseline: baseline.identity.clone(),
            candidate: candidate.identity.clone(),
            transitions,
        })
    }
}

fn classify_test_transition(
    baseline: Option<TestCaseOutcome>,
    candidate: Option<TestCaseOutcome>,
) -> TestComparisonCategory {
    match (baseline, candidate) {
        (Some(TestCaseOutcome::Passed | TestCaseOutcome::Skipped), Some(candidate))
            if candidate.is_failure() =>
        {
            TestComparisonCategory::Regression
        }
        (None, Some(candidate)) if candidate.is_failure() => TestComparisonCategory::NewFailure,
        (Some(baseline), Some(TestCaseOutcome::Passed)) if baseline.is_failure() => {
            TestComparisonCategory::NewPass
        }
        (Some(_), None) => TestComparisonCategory::Removed,
        _ => TestComparisonCategory::UnchangedOther,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestComparisonRequest {
    pub generation: u64,
    pub baseline: TestResultIdentity,
    pub candidate: TestResultIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestComparisonField {
    Baseline,
    Candidate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestComparisonPicker {
    pub baseline: Option<TestResultIdentity>,
    pub candidate: Option<TestResultIdentity>,
    pub active_field: TestComparisonField,
    pub cursor: Option<TestResultIdentity>,
    pub validation_error: Option<String>,
}

impl TestComparisonPicker {
    pub fn new(selected: Option<TestResultIdentity>, records: &[TestResultRecord]) -> Self {
        let baseline = selected.or_else(|| records.first().map(|record| record.identity.clone()));
        let candidate = records
            .iter()
            .find(|record| Some(&record.identity) != baseline.as_ref())
            .map(|record| record.identity.clone());
        Self {
            cursor: baseline.clone(),
            baseline,
            candidate,
            active_field: TestComparisonField::Baseline,
            validation_error: None,
        }
    }

    pub fn select(&mut self, records: &[TestResultRecord], delta: isize) {
        if records.is_empty() {
            self.cursor = None;
            return;
        }
        let current = self
            .cursor
            .as_ref()
            .and_then(|identity| {
                records
                    .iter()
                    .position(|record| &record.identity == identity)
            })
            .unwrap_or_default();
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current
                .saturating_add(delta as usize)
                .min(records.len() - 1)
        };
        self.cursor = records.get(next).map(|record| record.identity.clone());
        self.validation_error = None;
    }

    pub fn cycle_field(&mut self) {
        self.active_field = match self.active_field {
            TestComparisonField::Baseline => TestComparisonField::Candidate,
            TestComparisonField::Candidate => TestComparisonField::Baseline,
        };
        self.validation_error = None;
    }

    pub fn activate(&mut self) {
        match self.active_field {
            TestComparisonField::Baseline => self.baseline.clone_from(&self.cursor),
            TestComparisonField::Candidate => self.candidate.clone_from(&self.cursor),
        }
        self.validation_error = None;
    }

    pub fn preview(&self, generation: u64) -> Result<TestComparisonRequest, &'static str> {
        TestComparisonRequest::new(
            generation,
            self.baseline
                .clone()
                .ok_or("comparison baseline is unavailable")?,
            self.candidate
                .clone()
                .ok_or("comparison candidate is unavailable")?,
        )
    }
}

impl TestComparisonRequest {
    pub fn new(
        generation: u64,
        baseline: TestResultIdentity,
        candidate: TestResultIdentity,
    ) -> Result<Self, &'static str> {
        if generation == 0 || baseline == candidate {
            return Err("test comparison request is invalid");
        }
        Ok(Self {
            generation,
            baseline,
            candidate,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestComparisonPreview {
    pub request: TestComparisonRequest,
    pub argv: Vec<PathBuf>,
}

impl TestComparisonPreview {
    pub fn new(executable: PathBuf, request: TestComparisonRequest) -> Result<Self, &'static str> {
        if !absolute_normal_path(&executable) {
            return Err("resulttool executable identity is invalid");
        }
        let argv = vec![
            executable,
            "regression-file".into(),
            request.baseline.path.clone(),
            request.candidate.path.clone(),
        ];
        Ok(Self { request, argv })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TestComparisonState {
    #[default]
    NotSelected,
    Loading {
        request: TestComparisonRequest,
    },
    Available {
        request: TestComparisonRequest,
        comparison: TestComparison,
    },
    Partial {
        request: TestComparisonRequest,
        comparison: TestComparison,
        limitations: Vec<String>,
    },
    Failed {
        request: TestComparisonRequest,
        message: String,
    },
    Cancelled {
        request: TestComparisonRequest,
    },
    TimedOut {
        request: TestComparisonRequest,
    },
    Lost {
        request: TestComparisonRequest,
        message: String,
    },
}

impl TestComparisonState {
    pub fn request(&self) -> Option<&TestComparisonRequest> {
        match self {
            Self::NotSelected => None,
            Self::Loading { request }
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Failed { request, .. }
            | Self::Cancelled { request }
            | Self::TimedOut { request }
            | Self::Lost { request, .. } => Some(request),
        }
    }
}
