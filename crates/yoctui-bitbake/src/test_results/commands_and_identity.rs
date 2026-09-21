#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestResultOperation {
    Comparison(TestComparisonRequest),
    Junit(TestJunitExportRequest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResultCommandSpec {
    operation: TestResultOperation,
    executable: PathBuf,
    arguments: Vec<OsString>,
    current_directory: PathBuf,
    executable_identity: FileIdentity,
    result_identities: Vec<FileIdentity>,
    destination_parent: Option<PathBuf>,
}

impl TestResultCommandSpec {
    pub fn operation(&self) -> &TestResultOperation {
        &self.operation
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn current_directory(&self) -> &Path {
        &self.current_directory
    }

    fn revalidate(&self) -> Result<(), TestResultAdapterError> {
        self.executable_identity.revalidate_executable()?;
        for identity in &self.result_identities {
            identity.revalidate_result()?;
        }
        match &self.operation {
            TestResultOperation::Comparison(request) => {
                let expected = vec![
                    OsString::from("regression-file"),
                    request.baseline.path.as_os_str().to_owned(),
                    request.candidate.path.as_os_str().to_owned(),
                ];
                if self.arguments != expected || self.destination_parent.is_some() {
                    return Err(TestResultAdapterError::PreviewMismatch);
                }
            }
            TestResultOperation::Junit(request) => {
                let parent = validate_junit_destination(&request.destination)?;
                let expected = vec![
                    OsString::from("junit"),
                    request.result.path.as_os_str().to_owned(),
                    OsString::from("-j"),
                    request.destination.as_os_str().to_owned(),
                ];
                if self.arguments != expected || self.destination_parent.as_ref() != Some(&parent) {
                    return Err(TestResultAdapterError::PreviewMismatch);
                }
            }
        }
        Ok(())
    }

    pub fn comparison(&self) -> Option<TestComparisonRequest> {
        match &self.operation {
            TestResultOperation::Comparison(request) => Some(request.clone()),
            TestResultOperation::Junit(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileIdentity {
    path: PathBuf,
    size_bytes: u64,
    modified_at: SystemTime,
    fingerprint: Option<String>,
}

impl FileIdentity {
    fn capture_executable(path: &Path) -> Result<Self, TestResultAdapterError> {
        let metadata = safe_regular_file(path)
            .map_err(|_| TestResultAdapterError::UnsafeResultTool(path.into()))?;
        if !is_executable(&metadata) {
            return Err(TestResultAdapterError::UnsafeResultTool(path.into()));
        }
        Ok(Self {
            path: path.into(),
            size_bytes: metadata.len(),
            modified_at: metadata
                .modified()
                .map_err(|_| TestResultAdapterError::UnsafeResultTool(path.into()))?,
            fingerprint: None,
        })
    }

    fn capture_result(identity: &TestResultIdentity) -> Result<Self, TestResultAdapterError> {
        validate_result_identity(identity)?;
        Ok(Self {
            path: identity.path.clone(),
            size_bytes: identity.byte_size,
            modified_at: identity.modified_at,
            fingerprint: Some(identity.fingerprint.clone()),
        })
    }

    fn revalidate_executable(&self) -> Result<(), TestResultAdapterError> {
        let current = Self::capture_executable(&self.path)?;
        if current != *self {
            return Err(TestResultAdapterError::UnsafeResultTool(self.path.clone()));
        }
        Ok(())
    }

    fn revalidate_result(&self) -> Result<(), TestResultAdapterError> {
        let identity = TestResultIdentity::new(
            self.path.clone(),
            self.size_bytes,
            self.modified_at,
            self.fingerprint.clone().unwrap_or_default(),
        )
        .map_err(|_| TestResultAdapterError::StaleResult(self.path.clone()))?;
        validate_result_identity(&identity)
    }
}
