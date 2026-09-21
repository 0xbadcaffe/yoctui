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
    pub outcome: Option<TestSessionOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestSessionOutcome {
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Lost,
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
