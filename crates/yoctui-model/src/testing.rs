use crate::{BackgroundJobId, BuildRequest};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime},
};

pub const MAX_TEST_SELECTOR_BYTES: usize = 256;
pub const MAX_TEST_PARALLELISM_INPUT_BYTES: usize = 3;
pub const MAX_TEST_RESULT_PATHS: usize = 256;
pub const MAX_TEST_RESULTS: usize = 256;
pub const MAX_TEST_SUITES: usize = 512;
pub const MAX_TEST_CASES_PER_SUITE: usize = 4_096;
pub const MAX_TEST_METADATA: usize = 128;
pub const MAX_TEST_LIMITATIONS: usize = 128;
pub const MAX_TEST_TEXT_BYTES: usize = 4_096;
pub const MAX_TEST_FINGERPRINT_BYTES: usize = 256;

fn bounded_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TEST_SELECTOR_BYTES
        && !matches!(value, "." | "..")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
        })
}

fn absolute_normal_path(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && path.as_os_str().len() <= 4_096
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir | Component::CurDir))
}

fn bounded_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_TEST_TEXT_BYTES && !value.chars().any(char::is_control)
}

fn bounded_fingerprint(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TEST_FINGERPRINT_BYTES
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

pub fn test_result_paths_are_valid(paths: &[PathBuf]) -> bool {
    paths.len() <= MAX_TEST_RESULT_PATHS && paths.iter().all(|path| absolute_normal_path(path))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TestResultIdentity {
    pub path: PathBuf,
    pub byte_size: u64,
    pub modified_at: SystemTime,
    pub fingerprint: String,
}

impl TestResultIdentity {
    pub fn new(
        path: PathBuf,
        byte_size: u64,
        modified_at: SystemTime,
        fingerprint: String,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&path) || byte_size == 0 || !bounded_fingerprint(&fingerprint) {
            return Err("test result identity is invalid");
        }
        Ok(Self {
            path,
            byte_size,
            modified_at,
            fingerprint,
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(
            self.path.clone(),
            self.byte_size,
            self.modified_at,
            self.fingerprint.clone(),
        )
        .as_ref()
            == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TestCaseIdentity {
    pub suite: String,
    pub case: String,
}

impl TestCaseIdentity {
    pub fn new(suite: String, case: String) -> Result<Self, &'static str> {
        if !bounded_text(&suite) || !bounded_text(&case) {
            return Err("test case identity is invalid");
        }
        Ok(Self { suite, case })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.suite.clone(), self.case.clone()).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestCaseOutcome {
    Passed,
    Failed,
    Skipped,
    Error,
    Unknown,
}

impl TestCaseOutcome {
    fn is_failure(self) -> bool {
        matches!(self, Self::Failed | Self::Error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TestMetadata {
    pub key: String,
    pub value: String,
}

impl TestMetadata {
    pub fn new(key: String, value: String) -> Result<Self, &'static str> {
        if !bounded_text(&key) || !bounded_text(&value) {
            return Err("test metadata is invalid");
        }
        Ok(Self { key, value })
    }

    fn is_valid(&self) -> bool {
        Self::new(self.key.clone(), self.value.clone()).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCaseRecord {
    pub identity: TestCaseIdentity,
    pub outcome: TestCaseOutcome,
    pub duration: Option<Duration>,
    pub metadata: Vec<TestMetadata>,
    pub log_path: Option<PathBuf>,
}

impl TestCaseRecord {
    pub fn new(
        identity: TestCaseIdentity,
        outcome: TestCaseOutcome,
        duration: Option<Duration>,
        metadata: Vec<TestMetadata>,
        log_path: Option<PathBuf>,
    ) -> Result<(Self, Vec<String>), &'static str> {
        if log_path
            .as_deref()
            .is_some_and(|path| !absolute_normal_path(path))
        {
            return Err("test case log identity is invalid");
        }
        let (metadata, limitations) = normalize_test_metadata(metadata);
        Ok((
            Self {
                identity,
                outcome,
                duration,
                metadata,
                log_path,
            },
            limitations,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSuiteRecord {
    pub identity: String,
    pub duration: Option<Duration>,
    pub metadata: Vec<TestMetadata>,
    pub cases: Vec<TestCaseRecord>,
}

impl TestSuiteRecord {
    pub fn new(
        identity: String,
        duration: Option<Duration>,
        metadata: Vec<TestMetadata>,
        mut cases: Vec<TestCaseRecord>,
    ) -> Result<(Self, Vec<String>), &'static str> {
        if !bounded_text(&identity) || cases.iter().any(|case| case.identity.suite != identity) {
            return Err("test suite identity is invalid or mismatched");
        }
        let (metadata, mut limitations) = normalize_test_metadata(metadata);
        cases.sort_by(|left, right| left.identity.cmp(&right.identity));
        let before = cases.len();
        cases.dedup_by(|left, right| left.identity == right.identity);
        if cases.len() != before {
            limitations.push(format!(
                "ignored {} duplicate test cases in suite {identity}",
                before - cases.len()
            ));
        }
        if cases.len() > MAX_TEST_CASES_PER_SUITE {
            let dropped = cases.len() - MAX_TEST_CASES_PER_SUITE;
            cases.truncate(MAX_TEST_CASES_PER_SUITE);
            limitations.push(format!(
                "ignored {dropped} test cases beyond the per-suite bound"
            ));
        }
        Ok((
            Self {
                identity,
                duration,
                metadata,
                cases,
            },
            limitations,
        ))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TestOutcomeCounts {
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub errors: usize,
    pub unknown: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResultRecord {
    pub identity: TestResultIdentity,
    pub family: Option<TestFamily>,
    pub machine: Option<String>,
    pub image: Option<String>,
    pub revision: Option<String>,
    pub duration: Option<Duration>,
    pub metadata: Vec<TestMetadata>,
    pub suites: Vec<TestSuiteRecord>,
    pub originating_session: Option<TestSessionId>,
    pub limitations: Vec<String>,
}

impl TestResultRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: TestResultIdentity,
        family: Option<TestFamily>,
        machine: Option<String>,
        image: Option<String>,
        revision: Option<String>,
        duration: Option<Duration>,
        metadata: Vec<TestMetadata>,
        mut suites: Vec<TestSuiteRecord>,
        originating_session: Option<TestSessionId>,
        limitations: Vec<String>,
    ) -> (Self, Vec<String>) {
        let (metadata, mut normalization) = normalize_test_metadata(metadata);
        let mut normalize_optional = |label: &str, value: Option<String>| {
            value.and_then(|value| {
                if bounded_text(&value) {
                    Some(value)
                } else {
                    normalization.push(format!("ignored invalid {label} metadata"));
                    None
                }
            })
        };
        let machine = normalize_optional("machine", machine);
        let image = normalize_optional("image", image);
        let revision = normalize_optional("revision", revision);
        suites.sort_by(|left, right| left.identity.cmp(&right.identity));
        let before = suites.len();
        suites.dedup_by(|left, right| left.identity == right.identity);
        if suites.len() != before {
            normalization.push(format!(
                "ignored {} duplicate test suites",
                before - suites.len()
            ));
        }
        if suites.len() > MAX_TEST_SUITES {
            let dropped = suites.len() - MAX_TEST_SUITES;
            suites.truncate(MAX_TEST_SUITES);
            normalization.push(format!("ignored {dropped} suites beyond the result bound"));
        }
        let mut limitations = normalize_limitations(limitations);
        limitations.extend(normalization.iter().cloned());
        limitations = normalize_limitations(limitations);
        (
            Self {
                identity,
                family,
                machine,
                image,
                revision,
                duration,
                metadata,
                suites,
                originating_session,
                limitations,
            },
            normalization,
        )
    }

    pub fn counts(&self) -> TestOutcomeCounts {
        let mut counts = TestOutcomeCounts::default();
        for case in self.suites.iter().flat_map(|suite| &suite.cases) {
            match case.outcome {
                TestCaseOutcome::Passed => counts.passed += 1,
                TestCaseOutcome::Failed => counts.failed += 1,
                TestCaseOutcome::Skipped => counts.skipped += 1,
                TestCaseOutcome::Error => counts.errors += 1,
                TestCaseOutcome::Unknown => counts.unknown += 1,
            }
        }
        counts
    }

    pub fn is_valid(&self) -> bool {
        self.identity.is_valid()
            && self
                .machine
                .iter()
                .chain(self.image.iter())
                .chain(self.revision.iter())
                .all(|value| bounded_text(value))
            && self.metadata.iter().all(TestMetadata::is_valid)
            && self.suites.len() <= MAX_TEST_SUITES
            && self.suites.iter().all(|suite| {
                bounded_text(&suite.identity)
                    && suite.metadata.iter().all(TestMetadata::is_valid)
                    && suite.cases.len() <= MAX_TEST_CASES_PER_SUITE
                    && suite.cases.iter().all(|case| {
                        case.identity.is_valid()
                            && case.identity.suite == suite.identity
                            && case.metadata.iter().all(TestMetadata::is_valid)
                            && case.log_path.as_deref().is_none_or(absolute_normal_path)
                    })
            })
            && self.limitations.len() <= MAX_TEST_LIMITATIONS
            && self.limitations.iter().all(|value| bounded_text(value))
    }

    pub fn case(&self, identity: &TestCaseIdentity) -> Option<&TestCaseRecord> {
        self.suites
            .iter()
            .find(|suite| suite.identity == identity.suite)
            .and_then(|suite| suite.cases.iter().find(|case| &case.identity == identity))
    }
}

fn normalize_test_metadata(metadata: Vec<TestMetadata>) -> (Vec<TestMetadata>, Vec<String>) {
    let before = metadata.len();
    let mut exact = BTreeMap::new();
    for entry in metadata {
        exact.entry(entry.key).or_insert(entry.value);
    }
    let duplicate_count = before.saturating_sub(exact.len());
    let dropped = exact.len().saturating_sub(MAX_TEST_METADATA);
    let normalized = exact
        .into_iter()
        .take(MAX_TEST_METADATA)
        .map(|(key, value)| TestMetadata { key, value })
        .collect();
    let mut limitations = Vec::new();
    if duplicate_count > 0 {
        limitations.push(format!(
            "ignored {duplicate_count} duplicate metadata entries"
        ));
    }
    if dropped > 0 {
        limitations.push(format!(
            "ignored {dropped} metadata entries beyond the bound"
        ));
    }
    (normalized, limitations)
}

pub fn normalize_limitations(limitations: Vec<String>) -> Vec<String> {
    let mut exact = limitations
        .into_iter()
        .filter(|value| bounded_text(value))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    exact.truncate(MAX_TEST_LIMITATIONS);
    exact
}

pub fn normalize_test_results(
    mut records: Vec<TestResultRecord>,
    limitations: Vec<String>,
) -> (Vec<TestResultRecord>, Vec<String>) {
    records.sort_by(|left, right| left.identity.cmp(&right.identity));
    let before = records.len();
    records.dedup_by(|left, right| left.identity == right.identity);
    let duplicate_count = before - records.len();
    let dropped = records.len().saturating_sub(MAX_TEST_RESULTS);
    records.truncate(MAX_TEST_RESULTS);
    let mut limitations = normalize_limitations(limitations);
    if duplicate_count > 0 {
        limitations.push(format!(
            "ignored {duplicate_count} duplicate exact test results"
        ));
    }
    if dropped > 0 {
        limitations.push(format!("ignored {dropped} test results beyond the bound"));
    }
    (records, normalize_limitations(limitations))
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestJunitDestinationInspection {
    pub requested: PathBuf,
    pub canonical_parent: Option<PathBuf>,
    pub parent_exists: bool,
    pub parent_is_directory: bool,
    pub destination_exists: bool,
    pub destination_is_symlink: bool,
}

impl TestJunitDestinationInspection {
    pub fn validated_destination(&self) -> Result<PathBuf, &'static str> {
        let parent = self
            .canonical_parent
            .as_deref()
            .ok_or("JUnit destination parent is not canonical")?;
        let requested_parent = self
            .requested
            .parent()
            .ok_or("JUnit destination has no parent")?;
        if !absolute_normal_path(&self.requested)
            || self.requested.extension().and_then(|value| value.to_str()) != Some("xml")
            || !self.parent_exists
            || !self.parent_is_directory
            || self.destination_exists
            || self.destination_is_symlink
            || requested_parent != parent
        {
            return Err("JUnit destination is unsafe or would overwrite an existing path");
        }
        Ok(self.requested.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestJunitExportRequest {
    pub generation: u64,
    pub result: TestResultIdentity,
    pub destination: PathBuf,
}

impl TestJunitExportRequest {
    pub fn new(
        generation: u64,
        result: TestResultIdentity,
        inspection: &TestJunitDestinationInspection,
    ) -> Result<Self, &'static str> {
        if generation == 0 {
            return Err("JUnit export generation is invalid");
        }
        Ok(Self {
            generation,
            result,
            destination: inspection.validated_destination()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestJunitExportPreview {
    pub request: TestJunitExportRequest,
    pub argv: Vec<PathBuf>,
}

impl TestJunitExportPreview {
    pub fn new(executable: PathBuf, request: TestJunitExportRequest) -> Result<Self, &'static str> {
        if !absolute_normal_path(&executable) {
            return Err("resulttool executable identity is invalid");
        }
        let argv = vec![
            executable,
            "junit".into(),
            request.result.path.clone(),
            "-j".into(),
            request.destination.clone(),
        ];
        Ok(Self { request, argv })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TestJunitExportState {
    #[default]
    NotStarted,
    Inspecting {
        result: TestResultIdentity,
        destination: PathBuf,
    },
    Ready(TestJunitExportPreview),
    Running(TestJunitExportRequest),
    Succeeded(TestJunitExportRequest),
    Failed {
        request: TestJunitExportRequest,
        message: String,
    },
    Cancelled(TestJunitExportRequest),
    TimedOut(TestJunitExportRequest),
    Lost {
        request: TestJunitExportRequest,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestJunitExportDialog {
    pub result: TestResultIdentity,
    pub destination_input: String,
    pub validation_error: Option<String>,
}

impl TestJunitExportDialog {
    pub fn new(result: TestResultIdentity) -> Self {
        Self {
            result,
            destination_input: String::new(),
            validation_error: None,
        }
    }

    pub fn append(&mut self, character: char) {
        if !character.is_control()
            && self.destination_input.len() + character.len_utf8() <= MAX_TEST_TEXT_BYTES
        {
            self.destination_input.push(character);
            self.validation_error = None;
        }
    }

    pub fn backspace(&mut self) {
        self.destination_input.pop();
        self.validation_error = None;
    }

    pub fn lexical_destination(&self) -> Result<PathBuf, &'static str> {
        let path = PathBuf::from(&self.destination_input);
        if !absolute_normal_path(&path)
            || path.extension().and_then(|value| value.to_str()) != Some("xml")
        {
            return Err("JUnit destination must be a normalized absolute .xml path");
        }
        Ok(path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestFamily {
    OeSelftest,
    BitbakeSelftest,
    TestImage,
    TestSdk,
    TestSdkExt,
    Ptest,
}

impl TestFamily {
    pub const ALL: [Self; 6] = [
        Self::OeSelftest,
        Self::BitbakeSelftest,
        Self::TestImage,
        Self::TestSdk,
        Self::TestSdkExt,
        Self::Ptest,
    ];

    pub fn shifted(self, delta: isize) -> Self {
        let current = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or_default();
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current
                .saturating_add(delta as usize)
                .min(Self::ALL.len() - 1)
        };
        Self::ALL[next]
    }

    pub fn task(self) -> Option<&'static str> {
        match self {
            Self::TestImage | Self::Ptest => Some("testimage"),
            Self::TestSdk => Some("testsdk"),
            Self::TestSdkExt => Some("testsdkext"),
            Self::OeSelftest | Self::BitbakeSelftest => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::OeSelftest => "OE selftest",
            Self::BitbakeSelftest => "BitBake selftest",
            Self::TestImage => "Image runtime",
            Self::TestSdk => "Standard SDK",
            Self::TestSdkExt => "Extensible SDK",
            Self::Ptest => "Package tests",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TestExecutableCapability {
    #[default]
    NotInspected,
    Missing,
    Available(PathBuf),
    Failed(String),
}

impl TestExecutableCapability {
    pub fn executable(&self) -> Result<PathBuf, &'static str> {
        match self {
            Self::Available(path) if absolute_normal_path(path) => Ok(path.clone()),
            Self::Available(_) => Err("test executable identity is invalid"),
            Self::NotInspected => Err("test executable has not been inspected"),
            Self::Missing => Err("test executable is missing"),
            Self::Failed(_) => Err("test executable inspection failed"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PtestCapability {
    #[default]
    NotInspected,
    Configured,
    Unavailable(String),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TestCapability {
    pub oe_selftest: TestExecutableCapability,
    pub bitbake_selftest: TestExecutableCapability,
    pub ptest: PtestCapability,
}

impl TestCapability {
    pub fn executable_for(&self, family: TestFamily) -> Result<PathBuf, &'static str> {
        match family {
            TestFamily::OeSelftest => self.oe_selftest.executable(),
            TestFamily::BitbakeSelftest => self.bitbake_selftest.executable(),
            _ => Err("the selected test family is a managed BitBake task"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestSelectorScope {
    All,
    Selected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestLaunchDraft {
    pub family: TestFamily,
    pub machine: String,
    pub distro: String,
    pub image: String,
    pub scope: TestSelectorScope,
    pub selector: String,
    pub parallelism: u16,
    pub verbose: bool,
    pub skip_network: bool,
}

impl TestLaunchDraft {
    pub fn new(family: TestFamily, machine: String, distro: String, image: String) -> Self {
        Self {
            family,
            machine,
            distro,
            image,
            scope: TestSelectorScope::All,
            selector: String::new(),
            parallelism: 1,
            verbose: false,
            skip_network: false,
        }
    }

    pub fn preview(&self, capability: &TestCapability) -> Result<TestLaunchPreview, &'static str> {
        match self.family {
            TestFamily::OeSelftest | TestFamily::BitbakeSelftest => {
                let executable = capability.executable_for(self.family)?;
                if self.parallelism == 0 || self.parallelism > 256 {
                    return Err("test parallelism must be between 1 and 256");
                }
                let selector = match self.scope {
                    TestSelectorScope::All => None,
                    TestSelectorScope::Selected if bounded_token(&self.selector) => {
                        Some(self.selector.clone())
                    }
                    TestSelectorScope::Selected => {
                        return Err("selected test identity is invalid or unavailable");
                    }
                };
                if self.family == TestFamily::BitbakeSelftest
                    && self.scope == TestSelectorScope::All
                    && !self.selector.is_empty()
                {
                    return Err("BitBake selftest selector state is inconsistent");
                }
                TestSelftestRequest::new(
                    executable,
                    self.family,
                    selector,
                    self.parallelism,
                    self.verbose,
                    self.skip_network,
                )
                .map(TestLaunchPreview::Selftest)
            }
            family => {
                if !bounded_token(&self.machine)
                    || !bounded_token(&self.distro)
                    || !bounded_token(&self.image)
                {
                    return Err("test build identity must use bounded BitBake tokens");
                }
                if family == TestFamily::Ptest
                    && !matches!(capability.ptest, PtestCapability::Configured)
                {
                    return Err("ptest is not confirmed in the active image configuration");
                }
                let request = BuildRequest {
                    targets: vec![self.image.clone()],
                    task: family.task().map(str::to_owned),
                    force: false,
                };
                request
                    .validate()
                    .map_err(|_| "test BuildRequest is invalid")?;
                Ok(TestLaunchPreview::Build {
                    family,
                    machine: self.machine.clone(),
                    distro: self.distro.clone(),
                    image: self.image.clone(),
                    request,
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSelftestRequest {
    pub executable: PathBuf,
    pub family: TestFamily,
    pub selector: Option<String>,
    pub parallelism: u16,
    pub verbose: bool,
    pub skip_network: bool,
}

impl TestSelftestRequest {
    pub fn new(
        executable: PathBuf,
        family: TestFamily,
        selector: Option<String>,
        parallelism: u16,
        verbose: bool,
        skip_network: bool,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&executable)
            || !matches!(family, TestFamily::OeSelftest | TestFamily::BitbakeSelftest)
            || selector
                .as_deref()
                .is_some_and(|value| !bounded_token(value))
            || parallelism == 0
            || parallelism > 256
        {
            return Err("selftest request is invalid or exceeds its bound");
        }
        Ok(Self {
            executable,
            family,
            selector,
            parallelism,
            verbose,
            skip_network,
        })
    }

    pub fn argv(&self) -> Vec<PathBuf> {
        let mut argv = vec![self.executable.clone()];
        match self.family {
            TestFamily::OeSelftest => {
                if let Some(selector) = &self.selector {
                    argv.push("-r".into());
                    argv.push(selector.into());
                } else {
                    argv.push("-a".into());
                }
                argv.push("-j".into());
                argv.push(self.parallelism.to_string().into());
            }
            TestFamily::BitbakeSelftest => {
                if self.verbose {
                    argv.push("-v".into());
                }
                if let Some(selector) = &self.selector {
                    argv.push(selector.into());
                }
            }
            _ => {}
        }
        argv
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestLaunchPreview {
    Selftest(TestSelftestRequest),
    Build {
        family: TestFamily,
        machine: String,
        distro: String,
        image: String,
        request: BuildRequest,
    },
}

impl TestLaunchPreview {
    pub fn family(&self) -> TestFamily {
        match self {
            Self::Selftest(request) => request.family,
            Self::Build { family, .. } => *family,
        }
    }

    pub fn operation(&self) -> TestOperation {
        match self {
            Self::Selftest(request) => TestOperation::Selftest(request.clone()),
            Self::Build {
                family, request, ..
            } => TestOperation::Build {
                family: *family,
                request: request.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestLaunchField {
    Scope,
    Selector,
    Parallelism,
    Verbose,
    SkipNetwork,
}

impl TestLaunchField {
    fn fields(family: TestFamily) -> &'static [Self] {
        match family {
            TestFamily::OeSelftest => &[Self::Scope, Self::Selector, Self::Parallelism],
            TestFamily::BitbakeSelftest => &[
                Self::Scope,
                Self::Selector,
                Self::Verbose,
                Self::SkipNetwork,
            ],
            _ => &[],
        }
    }

    pub fn shifted(self, family: TestFamily, delta: isize) -> Self {
        let fields = Self::fields(family);
        let current = fields
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or_default();
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current
                .saturating_add(delta as usize)
                .min(fields.len().saturating_sub(1))
        };
        fields.get(next).copied().unwrap_or(self)
    }

    pub fn is_text(self) -> bool {
        matches!(self, Self::Selector | Self::Parallelism)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestLaunchDialog {
    pub draft: TestLaunchDraft,
    pub selected_field: Option<TestLaunchField>,
    pub editing: bool,
    pub parallelism_input: String,
    pub validation_error: Option<String>,
}

impl TestLaunchDialog {
    pub fn new(draft: TestLaunchDraft) -> Self {
        let selected_field = TestLaunchField::fields(draft.family).first().copied();
        Self {
            parallelism_input: draft.parallelism.to_string(),
            draft,
            selected_field,
            editing: false,
            validation_error: None,
        }
    }

    pub fn select(&mut self, delta: isize) {
        if let Some(field) = self.selected_field {
            self.selected_field = Some(field.shifted(self.draft.family, delta));
        }
        self.validation_error = None;
    }

    pub fn activate(&mut self) {
        match self.selected_field {
            Some(TestLaunchField::Scope) => {
                self.draft.scope = match self.draft.scope {
                    TestSelectorScope::All => TestSelectorScope::Selected,
                    TestSelectorScope::Selected => TestSelectorScope::All,
                };
                if self.draft.scope == TestSelectorScope::All {
                    self.draft.selector.clear();
                }
            }
            Some(TestLaunchField::Selector | TestLaunchField::Parallelism) => {
                self.editing = true;
            }
            Some(TestLaunchField::Verbose) => self.draft.verbose = !self.draft.verbose,
            Some(TestLaunchField::SkipNetwork) => {
                self.draft.skip_network = !self.draft.skip_network;
            }
            None => {}
        }
        self.validation_error = None;
    }

    pub fn append(&mut self, character: char) {
        if !self.editing || character.is_control() {
            return;
        }
        match self.selected_field {
            Some(TestLaunchField::Selector)
                if self.draft.selector.len() + character.len_utf8() <= MAX_TEST_SELECTOR_BYTES =>
            {
                self.draft.selector.push(character);
            }
            Some(TestLaunchField::Parallelism)
                if character.is_ascii_digit()
                    && self.parallelism_input.len() < MAX_TEST_PARALLELISM_INPUT_BYTES =>
            {
                self.parallelism_input.push(character);
            }
            _ => {}
        }
        self.validation_error = None;
    }

    pub fn backspace(&mut self) {
        if !self.editing {
            return;
        }
        match self.selected_field {
            Some(TestLaunchField::Selector) => {
                self.draft.selector.pop();
            }
            Some(TestLaunchField::Parallelism) => {
                self.parallelism_input.pop();
            }
            _ => {}
        }
        self.validation_error = None;
    }

    pub fn finish_edit(&mut self) {
        if self.selected_field == Some(TestLaunchField::Parallelism) {
            match self.parallelism_input.parse::<u16>() {
                Ok(value @ 1..=256) => self.draft.parallelism = value,
                _ => {
                    self.validation_error = Some("Parallelism must be between 1 and 256.".into());
                    return;
                }
            }
        }
        self.editing = false;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestOperation {
    Selftest(TestSelftestRequest),
    Build {
        family: TestFamily,
        request: BuildRequest,
    },
}

impl TestOperation {
    pub fn family(&self) -> TestFamily {
        match self {
            Self::Selftest(request) => request.family,
            Self::Build { family, .. } => *family,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TestSessionId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSession {
    pub id: TestSessionId,
    pub background_job_id: Option<BackgroundJobId>,
    pub operation: TestOperation,
    pub exit_code: Option<i32>,
    pub result_paths: Vec<PathBuf>,
    pub error_detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSessionTerminal {
    pub id: TestSessionId,
    pub exit_code: Option<i32>,
    pub result_paths: Vec<PathBuf>,
    pub finished_at: SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability() -> TestCapability {
        TestCapability {
            oe_selftest: TestExecutableCapability::Available("/workspace/oe-selftest".into()),
            bitbake_selftest: TestExecutableCapability::Available(
                "/workspace/bitbake-selftest".into(),
            ),
            ptest: PtestCapability::Configured,
        }
    }

    #[test]
    fn test_workflow_previews_exact_selftest_and_build_operations() {
        let mut oe = TestLaunchDraft::new(
            TestFamily::OeSelftest,
            "qemux86-64".into(),
            "poky".into(),
            "core-image-minimal".into(),
        );
        oe.scope = TestSelectorScope::Selected;
        oe.selector = "tinfoil.TinfoilTests.test_getvar".into();
        oe.parallelism = 8;
        let TestLaunchPreview::Selftest(request) = oe.preview(&capability()).unwrap() else {
            panic!("selftest preview");
        };
        assert_eq!(
            request.argv(),
            [
                PathBuf::from("/workspace/oe-selftest"),
                "-r".into(),
                "tinfoil.TinfoilTests.test_getvar".into(),
                "-j".into(),
                "8".into(),
            ]
        );

        for (family, task) in [
            (TestFamily::TestImage, "testimage"),
            (TestFamily::TestSdk, "testsdk"),
            (TestFamily::TestSdkExt, "testsdkext"),
            (TestFamily::Ptest, "testimage"),
        ] {
            let TestLaunchPreview::Build { request, .. } = TestLaunchDraft::new(
                family,
                "qemux86-64".into(),
                "poky".into(),
                "core-image-minimal".into(),
            )
            .preview(&capability())
            .unwrap() else {
                panic!("build preview");
            };
            assert_eq!(request.task.as_deref(), Some(task));
            assert_eq!(request.targets, ["core-image-minimal"]);
        }
    }

    #[test]
    fn test_workflow_dialog_bounds_editing_and_reports_invalid_state() {
        let mut dialog = TestLaunchDialog::new(TestLaunchDraft::new(
            TestFamily::OeSelftest,
            "qemux86-64".into(),
            "poky".into(),
            "core-image-minimal".into(),
        ));
        dialog.activate();
        dialog.select(1);
        dialog.activate();
        for _ in 0..(MAX_TEST_SELECTOR_BYTES + 10) {
            dialog.append('x');
        }
        assert_eq!(dialog.draft.selector.len(), MAX_TEST_SELECTOR_BYTES);
        dialog.finish_edit();
        dialog.select(1);
        dialog.activate();
        dialog.parallelism_input.clear();
        for character in "999".chars() {
            dialog.append(character);
        }
        dialog.finish_edit();
        assert!(dialog.editing);
        assert!(dialog.validation_error.is_some());
    }

    #[test]
    fn test_workflow_rejects_missing_tools_bad_tokens_and_unconfigured_ptest() {
        let missing = TestCapability::default();
        let oe = TestLaunchDraft::new(
            TestFamily::OeSelftest,
            "qemux86-64".into(),
            "poky".into(),
            "image".into(),
        );
        assert_eq!(
            oe.preview(&missing),
            Err("test executable has not been inspected")
        );
        let bad = TestLaunchDraft::new(
            TestFamily::TestImage,
            "../machine".into(),
            "poky".into(),
            "image".into(),
        );
        assert!(bad.preview(&capability()).is_err());
        let ptest = TestLaunchDraft::new(
            TestFamily::Ptest,
            "qemux86-64".into(),
            "poky".into(),
            "image".into(),
        );
        assert!(ptest.preview(&missing).is_err());
    }

    fn result_identity(name: &str, fingerprint: &str) -> TestResultIdentity {
        TestResultIdentity::new(
            format!("/build/testresults/{name}/testresults.json").into(),
            1_024,
            SystemTime::UNIX_EPOCH,
            fingerprint.into(),
        )
        .unwrap()
    }

    fn case(suite: &str, name: &str, outcome: TestCaseOutcome) -> TestCaseRecord {
        TestCaseRecord::new(
            TestCaseIdentity::new(suite.into(), name.into()).unwrap(),
            outcome,
            Some(Duration::from_millis(10)),
            Vec::new(),
            Some(format!("/build/logs/{suite}-{name}.log").into()),
        )
        .unwrap()
        .0
    }

    fn result(name: &str, fingerprint: &str, cases: Vec<TestCaseRecord>) -> TestResultRecord {
        let (suite, _) = TestSuiteRecord::new("suite".into(), None, Vec::new(), cases).unwrap();
        TestResultRecord::new(
            result_identity(name, fingerprint),
            Some(TestFamily::TestImage),
            Some("qemux86-64".into()),
            Some("core-image-minimal".into()),
            Some("rev-1".into()),
            None,
            Vec::new(),
            vec![suite],
            Some(TestSessionId(1)),
            Vec::new(),
        )
        .0
    }

    #[test]
    fn test_results_normalize_exact_records_metadata_duplicates_and_bounds() {
        let duplicate = case("suite", "same", TestCaseOutcome::Passed);
        let (suite, suite_limitations) = TestSuiteRecord::new(
            "suite".into(),
            None,
            vec![
                TestMetadata::new("z".into(), "last".into()).unwrap(),
                TestMetadata::new("z".into(), "ignored".into()).unwrap(),
                TestMetadata::new("a".into(), "first".into()).unwrap(),
            ],
            vec![duplicate.clone(), duplicate],
        )
        .unwrap();
        assert_eq!(suite.metadata[0].key, "a");
        assert_eq!(suite.cases.len(), 1);
        assert!(
            suite_limitations
                .iter()
                .any(|value| value.contains("duplicate"))
        );

        let record = TestResultRecord::new(
            result_identity("one", "abc123"),
            None,
            Some("\n".into()),
            None,
            None,
            None,
            Vec::new(),
            vec![suite],
            None,
            vec!["adapter skipped malformed record".into()],
        )
        .0;
        assert_eq!(record.machine, None);
        assert_eq!(record.counts().passed, 1);
        assert!(
            record
                .limitations
                .iter()
                .any(|value| value.contains("invalid machine"))
        );

        let (records, limitations) = normalize_test_results(
            vec![record.clone(), record],
            vec!["adapter skipped malformed record".into()],
        );
        assert_eq!(records.len(), 1);
        assert!(
            limitations
                .iter()
                .any(|value| value.contains("duplicate exact test results"))
        );
        assert!(
            TestResultIdentity::new(
                "relative.json".into(),
                1,
                SystemTime::UNIX_EPOCH,
                "abc".into()
            )
            .is_err()
        );
    }

    #[test]
    fn test_results_comparison_uses_exact_suite_case_status_transitions() {
        let baseline = result(
            "baseline",
            "base",
            vec![
                case("suite", "regression", TestCaseOutcome::Passed),
                case("suite", "fixed", TestCaseOutcome::Failed),
                case("suite", "removed", TestCaseOutcome::Skipped),
                case("suite", "same", TestCaseOutcome::Passed),
            ],
        );
        let candidate = result(
            "candidate",
            "candidate",
            vec![
                case("suite", "regression", TestCaseOutcome::Error),
                case("suite", "fixed", TestCaseOutcome::Passed),
                case("suite", "new-failure", TestCaseOutcome::Failed),
                case("suite", "same", TestCaseOutcome::Passed),
            ],
        );
        let comparison = TestComparison::between(&baseline, &candidate).unwrap();
        let categories = comparison
            .transitions
            .iter()
            .map(|transition| (transition.identity.case.as_str(), transition.category))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(categories["regression"], TestComparisonCategory::Regression);
        assert_eq!(categories["fixed"], TestComparisonCategory::NewPass);
        assert_eq!(
            categories["new-failure"],
            TestComparisonCategory::NewFailure
        );
        assert_eq!(categories["removed"], TestComparisonCategory::Removed);
        assert_eq!(categories["same"], TestComparisonCategory::UnchangedOther);
        assert!(TestComparison::between(&baseline, &baseline).is_err());
    }

    #[test]
    fn test_results_import_and_junit_require_exact_non_overwriting_paths() {
        assert!(TestResultImportRequest::new(1, vec!["relative".into()]).is_err());
        let request = TestResultImportRequest::new(2, vec!["/build/results".into()]).unwrap();
        assert_eq!(request.roots, [PathBuf::from("/build/results")]);

        let result = result_identity("candidate", "candidate");
        let valid = TestJunitDestinationInspection {
            requested: "/exports/candidate.xml".into(),
            canonical_parent: Some("/exports".into()),
            parent_exists: true,
            parent_is_directory: true,
            destination_exists: false,
            destination_is_symlink: false,
        };
        let export = TestJunitExportRequest::new(1, result.clone(), &valid).unwrap();
        let preview = TestJunitExportPreview::new("/workspace/resulttool".into(), export).unwrap();
        assert_eq!(
            preview.argv,
            [
                PathBuf::from("/workspace/resulttool"),
                "junit".into(),
                result.path,
                "-j".into(),
                "/exports/candidate.xml".into(),
            ]
        );

        for invalid in [
            TestJunitDestinationInspection {
                requested: "relative.xml".into(),
                canonical_parent: Some("/exports".into()),
                ..valid.clone()
            },
            TestJunitDestinationInspection {
                destination_exists: true,
                ..valid.clone()
            },
            TestJunitDestinationInspection {
                canonical_parent: Some("/other".into()),
                ..valid.clone()
            },
        ] {
            assert!(invalid.validated_destination().is_err());
        }
    }
}
