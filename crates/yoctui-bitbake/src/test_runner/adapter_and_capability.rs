use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, SystemTime},
};

use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
    time::Instant,
};
use yoctui_model::{
    PtestCapability, TestCapability, TestExecutableCapability, TestFamily, TestOutputStream,
    TestSelftestRequest,
};

use crate::output_text;

const MAX_TEST_RUNNER_PATH_DIRECTORIES: usize = 256;
const MAX_TEST_RUNNER_LINE_BYTES: usize = 64 * 1024;
const TEST_RUNNER_EVENT_CHANNEL_CAPACITY: usize = 256;
const TEST_RUNNER_OPERATION_TIMEOUT: Duration = Duration::from_secs(12 * 60 * 60);
const TEST_RUNNER_SPAWN_ATTEMPTS: usize = 4;
const TEST_RUNNER_SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);

#[cfg(unix)]
fn is_transient_test_runner_spawn_error(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ETXTBSY)
}

#[cfg(not(unix))]
fn is_transient_test_runner_spawn_error(_error: &io::Error) -> bool {
    false
}

async fn spawn_test_runner_process(process: &mut Command) -> io::Result<Child> {
    for attempt in 1..=TEST_RUNNER_SPAWN_ATTEMPTS {
        match process.spawn() {
            Ok(child) => return Ok(child),
            Err(error)
                if attempt < TEST_RUNNER_SPAWN_ATTEMPTS
                    && is_transient_test_runner_spawn_error(&error) =>
            {
                tokio::time::sleep(TEST_RUNNER_SPAWN_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the bounded Testing process spawn loop always returns")
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TestRunnerAdapterError {
    #[error("Testing PATH directory is unsafe: {0}")]
    UnsafePathDirectory(PathBuf),
    #[error("Testing executable is unsafe or unavailable: {0}")]
    UnsafeExecutable(PathBuf),
    #[error("Testing build directory is unsafe: {0}")]
    UnsafeBuildDirectory(PathBuf),
    #[error("Testing request is invalid: {0}")]
    InvalidRequest(String),
    #[error("Testing executable inspection failed: {0}")]
    Inspection(String),
    #[error("Testing executable identity changed before launch: {0}")]
    StaleExecutable(PathBuf),
    #[error("a Testing process or unconsumed event is already active")]
    Busy,
    #[error("could not start Testing process: {0}")]
    Spawn(String),
    #[error("Testing process stream is unavailable: {0:?}")]
    StreamUnavailable(TestOutputStream),
    #[error("Testing runner is not active")]
    NotRunning,
    #[error("Testing process control failed: {0}")]
    ProcessControl(String),
}

#[derive(Debug, Clone)]
pub struct TestRunnerCapabilityInspector {
    path_directories: Vec<PathBuf>,
    ptest: PtestCapability,
}

impl TestRunnerCapabilityInspector {
    pub fn new(path_directories: Vec<PathBuf>, ptest: PtestCapability) -> Self {
        Self {
            path_directories,
            ptest,
        }
    }

    pub fn inspect(&self) -> TestCapability {
        let directories = match validate_path_directories(&self.path_directories) {
            Ok(directories) => directories,
            Err(error) => {
                let message = error.to_string();
                return TestCapability {
                    oe_selftest: TestExecutableCapability::Failed(message.clone()),
                    bitbake_selftest: TestExecutableCapability::Failed(message),
                    ptest: self.ptest.clone(),
                };
            }
        };
        TestCapability {
            oe_selftest: discover_executable(&directories, "oe-selftest").map_or_else(
                TestExecutableCapability::Failed,
                |path| {
                    path.map_or(
                        TestExecutableCapability::Missing,
                        TestExecutableCapability::Available,
                    )
                },
            ),
            bitbake_selftest: discover_executable(&directories, "bitbake-selftest").map_or_else(
                TestExecutableCapability::Failed,
                |path| {
                    path.map_or(
                        TestExecutableCapability::Missing,
                        TestExecutableCapability::Available,
                    )
                },
            ),
            ptest: self.ptest.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestRunnerAdapter {
    build_directory: PathBuf,
    path_directories: Vec<PathBuf>,
    ptest: PtestCapability,
}

impl TestRunnerAdapter {
    pub fn new(
        build_directory: PathBuf,
        path_directories: Vec<PathBuf>,
        ptest: PtestCapability,
    ) -> Self {
        Self {
            build_directory,
            path_directories,
            ptest,
        }
    }

    pub fn capability(&self) -> TestCapability {
        TestRunnerCapabilityInspector::new(self.path_directories.clone(), self.ptest.clone())
            .inspect()
    }

    pub fn command(
        &self,
        request: &TestSelftestRequest,
    ) -> Result<TestCommandSpec, TestRunnerAdapterError> {
        let directories = validate_path_directories(&self.path_directories)?;
        let build_directory = validate_directory(
            &self.build_directory,
            TestRunnerAdapterError::UnsafeBuildDirectory(self.build_directory.clone()),
        )?;
        let expected_name = match request.family {
            TestFamily::OeSelftest => "oe-selftest",
            TestFamily::BitbakeSelftest => "bitbake-selftest",
            _ => {
                return Err(TestRunnerAdapterError::InvalidRequest(
                    "managed BitBake test families do not use the selftest runner".into(),
                ));
            }
        };
        if request.family == TestFamily::OeSelftest && (request.verbose || request.skip_network) {
            return Err(TestRunnerAdapterError::InvalidRequest(
                "OE selftest cannot carry BitBake-selftest-only choices".into(),
            ));
        }
        let discovered = discover_executable(&directories, expected_name)
            .map_err(TestRunnerAdapterError::Inspection)?
            .ok_or_else(|| TestRunnerAdapterError::UnsafeExecutable(request.executable.clone()))?;
        if discovered != request.executable {
            return Err(TestRunnerAdapterError::UnsafeExecutable(
                request.executable.clone(),
            ));
        }
        let reconstructed = TestSelftestRequest::new(
            request.executable.clone(),
            request.family,
            request.selector.clone(),
            request.parallelism,
            request.verbose,
            request.skip_network,
        )
        .map_err(|message| TestRunnerAdapterError::InvalidRequest(message.into()))?;
        if &reconstructed != request {
            return Err(TestRunnerAdapterError::InvalidRequest(
                "selftest request is not canonical".into(),
            ));
        }
        let argv = request.argv();
        if argv.first() != Some(&request.executable) {
            return Err(TestRunnerAdapterError::InvalidRequest(
                "selftest argv does not retain executable identity".into(),
            ));
        }
        let environment = test_child_environment(request);
        Ok(TestCommandSpec {
            request: request.clone(),
            executable: request.executable.clone(),
            arguments: argv
                .into_iter()
                .skip(1)
                .map(PathBuf::into_os_string)
                .collect(),
            current_directory: build_directory,
            environment,
            executable_identity: ExecutableIdentity::capture(&request.executable)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExecutableIdentity {
    canonical_path: PathBuf,
    size_bytes: u64,
    modified_at: SystemTime,
}

impl ExecutableIdentity {
    fn capture(path: &Path) -> Result<Self, TestRunnerAdapterError> {
        let metadata = safe_executable_metadata(path)?;
        let canonical_path = fs::canonicalize(path)
            .map_err(|_| TestRunnerAdapterError::UnsafeExecutable(path.into()))?;
        if canonical_path != path {
            return Err(TestRunnerAdapterError::UnsafeExecutable(path.into()));
        }
        Ok(Self {
            canonical_path,
            size_bytes: metadata.len(),
            modified_at: metadata
                .modified()
                .map_err(|_| TestRunnerAdapterError::UnsafeExecutable(path.into()))?,
        })
    }

    fn revalidate(&self) -> Result<(), TestRunnerAdapterError> {
        let current = Self::capture(&self.canonical_path)?;
        if current != *self {
            return Err(TestRunnerAdapterError::StaleExecutable(
                self.canonical_path.clone(),
            ));
        }
        Ok(())
    }
}
