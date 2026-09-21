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
    is_bounded_identifier(value, MAX_TEST_SELECTOR_BYTES)
}

fn absolute_normal_path(path: &Path) -> bool {
    is_absolute_normal_path_within(path, 4_096)
}

fn bounded_text(value: &str) -> bool {
    is_bounded_plain_text(value, MAX_TEST_TEXT_BYTES)
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
