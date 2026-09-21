use std::{
    collections::{BTreeSet, VecDeque},
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, SystemTime},
};

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
    time::Instant,
};
use yoctui_model::{
    ResultToolCapability, TestCaseIdentity, TestCaseOutcome, TestCaseRecord, TestComparisonPreview,
    TestComparisonRequest, TestFamily, TestJunitDestinationInspection, TestJunitExportPreview,
    TestJunitExportRequest, TestMetadata, TestOutputStream, TestResultIdentity,
    TestResultImportRequest, TestResultRecord, TestSuiteRecord, normalize_test_results,
};

use crate::{
    output_text,
    test_runner::{discover_executable, validate_path_directories},
};

const MAX_RESULT_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_RESULT_SCAN_DIRECTORIES: usize = 1_024;
const MAX_RESULT_SCAN_ENTRIES: usize = 16_384;
const MAX_RESULT_FILES: usize = 256;
const MAX_RESULTTOOL_LINE_BYTES: usize = 64 * 1024;
const RESULTTOOL_EVENT_CHANNEL_CAPACITY: usize = 256;
const RESULTTOOL_OPERATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TestResultAdapterError {
    #[error("resulttool capability inspection failed: {0}")]
    Capability(String),
    #[error("test-result root is unsafe or unavailable: {0}")]
    UnsafeRoot(PathBuf),
    #[error("test-result file is unsafe or unavailable: {0}")]
    UnsafeResult(PathBuf),
    #[error("test-result input exceeds a safety bound: {0}")]
    Bound(String),
    #[error("test-result JSON is malformed: {0}")]
    Malformed(String),
    #[error("resulttool executable identity is stale or unsafe: {0}")]
    UnsafeResultTool(PathBuf),
    #[error("test-result identity changed before operation: {0}")]
    StaleResult(PathBuf),
    #[error("resulttool preview does not match its exact reconstructed command")]
    PreviewMismatch,
    #[error("JUnit destination is unsafe or would overwrite data: {0}")]
    UnsafeDestination(PathBuf),
    #[error("a resulttool process or unconsumed event is already active")]
    Busy,
    #[error("could not start resulttool: {0}")]
    Spawn(String),
    #[error("resulttool process stream is unavailable: {0:?}")]
    StreamUnavailable(TestOutputStream),
    #[error("resulttool runner is not active")]
    NotRunning,
    #[error("resulttool process control failed: {0}")]
    ProcessControl(String),
}

#[derive(Debug, Clone)]
pub struct ResultToolCapabilityInspector {
    path_directories: Vec<PathBuf>,
}

impl ResultToolCapabilityInspector {
    pub fn new(path_directories: Vec<PathBuf>) -> Self {
        Self { path_directories }
    }

    pub fn inspect(&self) -> ResultToolCapability {
        let directories = match validate_path_directories(&self.path_directories) {
            Ok(directories) => directories,
            Err(error) => return ResultToolCapability::Failed(error.to_string()),
        };
        discover_executable(&directories, "resulttool").map_or_else(
            ResultToolCapability::Failed,
            |path| {
                path.map_or(
                    ResultToolCapability::Missing,
                    ResultToolCapability::Available,
                )
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResultImportResponse {
    pub request: TestResultImportRequest,
    pub records: Vec<TestResultRecord>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TestResultAdapter {
    path_directories: Vec<PathBuf>,
}

impl TestResultAdapter {
    pub fn new(path_directories: Vec<PathBuf>) -> Self {
        Self { path_directories }
    }

    pub fn capability(&self) -> ResultToolCapability {
        ResultToolCapabilityInspector::new(self.path_directories.clone()).inspect()
    }

    pub fn inspect_junit_destination(
        &self,
        destination: PathBuf,
    ) -> TestJunitDestinationInspection {
        let destination_metadata = fs::symlink_metadata(&destination).ok();
        let parent = destination.parent().map(Path::to_path_buf);
        let parent_metadata = parent
            .as_deref()
            .and_then(|path| fs::symlink_metadata(path).ok());
        TestJunitDestinationInspection {
            requested: destination,
            canonical_parent: parent
                .as_deref()
                .and_then(|path| fs::canonicalize(path).ok()),
            parent_exists: parent_metadata.is_some(),
            parent_is_directory: parent_metadata
                .as_ref()
                .is_some_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink()),
            destination_exists: destination_metadata.is_some(),
            destination_is_symlink: destination_metadata
                .as_ref()
                .is_some_and(|metadata| metadata.file_type().is_symlink()),
        }
    }

    pub fn import(
        &self,
        request: &TestResultImportRequest,
    ) -> Result<TestResultImportResponse, TestResultAdapterError> {
        let validated = TestResultImportRequest::new(request.generation, request.roots.clone())
            .map_err(|message| TestResultAdapterError::Malformed(message.into()))?;
        if &validated != request {
            return Err(TestResultAdapterError::Malformed(
                "test-result import request is not canonical".into(),
            ));
        }
        let paths = collect_result_files(&request.roots)?;
        let mut records = Vec::new();
        let mut limitations = Vec::new();
        for path in paths {
            match parse_result_file(&path) {
                Ok((record, file_limitations)) => {
                    records.push(record);
                    limitations.extend(file_limitations);
                }
                Err(error @ TestResultAdapterError::Bound(_))
                | Err(error @ TestResultAdapterError::Malformed(_)) => {
                    limitations.push(format!("skipped {}: {error}", path.display()));
                }
                Err(error) => return Err(error),
            }
        }
        let (records, limitations) = normalize_test_results(records, limitations);
        Ok(TestResultImportResponse {
            request: request.clone(),
            records,
            limitations,
        })
    }

    pub fn comparison_command(
        &self,
        preview: &TestComparisonPreview,
        baseline: &TestResultRecord,
        candidate: &TestResultRecord,
    ) -> Result<TestResultCommandSpec, TestResultAdapterError> {
        let executable = self.resulttool_executable()?;
        validate_result_identity(&baseline.identity)?;
        validate_result_identity(&candidate.identity)?;
        if preview.request.baseline != baseline.identity
            || preview.request.candidate != candidate.identity
        {
            return Err(TestResultAdapterError::PreviewMismatch);
        }
        let expected = TestComparisonPreview::new(executable.clone(), preview.request.clone())
            .map_err(|_| TestResultAdapterError::PreviewMismatch)?;
        if expected != *preview {
            return Err(TestResultAdapterError::PreviewMismatch);
        }
        Ok(TestResultCommandSpec {
            operation: TestResultOperation::Comparison(preview.request.clone()),
            executable: executable.clone(),
            arguments: expected
                .argv
                .iter()
                .skip(1)
                .map(|value| value.as_os_str().to_owned())
                .collect(),
            current_directory: baseline
                .identity
                .path
                .parent()
                .expect("validated result has a parent")
                .into(),
            executable_identity: FileIdentity::capture_executable(&executable)?,
            result_identities: vec![
                FileIdentity::capture_result(&baseline.identity)?,
                FileIdentity::capture_result(&candidate.identity)?,
            ],
            destination_parent: None,
        })
    }

    pub fn junit_command(
        &self,
        preview: &TestJunitExportPreview,
        result: &TestResultRecord,
    ) -> Result<TestResultCommandSpec, TestResultAdapterError> {
        let executable = self.resulttool_executable()?;
        validate_result_identity(&result.identity)?;
        if preview.request.result != result.identity {
            return Err(TestResultAdapterError::PreviewMismatch);
        }
        let parent = validate_junit_destination(&preview.request.destination)?;
        let expected = TestJunitExportPreview::new(executable.clone(), preview.request.clone())
            .map_err(|_| TestResultAdapterError::PreviewMismatch)?;
        if expected != *preview {
            return Err(TestResultAdapterError::PreviewMismatch);
        }
        Ok(TestResultCommandSpec {
            operation: TestResultOperation::Junit(preview.request.clone()),
            executable: executable.clone(),
            arguments: expected
                .argv
                .iter()
                .skip(1)
                .map(|value| value.as_os_str().to_owned())
                .collect(),
            current_directory: parent.clone(),
            executable_identity: FileIdentity::capture_executable(&executable)?,
            result_identities: vec![FileIdentity::capture_result(&result.identity)?],
            destination_parent: Some(parent),
        })
    }

    fn resulttool_executable(&self) -> Result<PathBuf, TestResultAdapterError> {
        match self.capability() {
            ResultToolCapability::Available(path) => Ok(path),
            ResultToolCapability::Missing => Err(TestResultAdapterError::Capability(
                "resulttool is missing".into(),
            )),
            ResultToolCapability::NotInspected => Err(TestResultAdapterError::Capability(
                "resulttool was not inspected".into(),
            )),
            ResultToolCapability::Failed(message) => {
                Err(TestResultAdapterError::Capability(message))
            }
        }
    }
}
