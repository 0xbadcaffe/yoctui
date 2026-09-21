#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCommandSpec {
    request: TestSelftestRequest,
    executable: PathBuf,
    arguments: Vec<OsString>,
    current_directory: PathBuf,
    environment: BTreeMap<OsString, OsString>,
    executable_identity: ExecutableIdentity,
}

impl TestCommandSpec {
    pub fn request(&self) -> &TestSelftestRequest {
        &self.request
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

    pub fn environment(&self) -> &BTreeMap<OsString, OsString> {
        &self.environment
    }

    fn revalidate(&self) -> Result<(), TestRunnerAdapterError> {
        self.executable_identity.revalidate()?;
        let build = validate_directory(
            &self.current_directory,
            TestRunnerAdapterError::UnsafeBuildDirectory(self.current_directory.clone()),
        )?;
        if build != self.current_directory {
            return Err(TestRunnerAdapterError::UnsafeBuildDirectory(
                self.current_directory.clone(),
            ));
        }
        let reconstructed = TestSelftestRequest::new(
            self.executable.clone(),
            self.request.family,
            self.request.selector.clone(),
            self.request.parallelism,
            self.request.verbose,
            self.request.skip_network,
        )
        .map_err(|message| TestRunnerAdapterError::InvalidRequest(message.into()))?;
        let argv = reconstructed.argv();
        let arguments = argv
            .into_iter()
            .skip(1)
            .map(PathBuf::into_os_string)
            .collect::<Vec<_>>();
        if reconstructed != self.request || arguments != self.arguments {
            return Err(TestRunnerAdapterError::InvalidRequest(
                "Testing command no longer matches its typed request".into(),
            ));
        }
        let expected_environment = test_child_environment(&self.request);
        if self.environment != expected_environment {
            return Err(TestRunnerAdapterError::InvalidRequest(
                "Testing child environment is inconsistent".into(),
            ));
        }
        Ok(())
    }
}

fn test_child_environment(request: &TestSelftestRequest) -> BTreeMap<OsString, OsString> {
    if request.family == TestFamily::BitbakeSelftest && request.skip_network {
        BTreeMap::from([(OsString::from("BB_SKIP_NETTESTS"), OsString::from("yes"))])
    } else {
        BTreeMap::new()
    }
}

pub(crate) fn validate_path_directories(
    directories: &[PathBuf],
) -> Result<Vec<PathBuf>, TestRunnerAdapterError> {
    if directories.is_empty() || directories.len() > MAX_TEST_RUNNER_PATH_DIRECTORIES {
        return Err(TestRunnerAdapterError::UnsafePathDirectory(PathBuf::new()));
    }
    let mut validated = Vec::with_capacity(directories.len());
    for directory in directories {
        if !directory.is_absolute() {
            return Err(TestRunnerAdapterError::UnsafePathDirectory(
                directory.clone(),
            ));
        }
        let canonical = fs::canonicalize(directory)
            .map_err(|_| TestRunnerAdapterError::UnsafePathDirectory(directory.clone()))?;
        if !fs::metadata(&canonical)
            .map_err(|_| TestRunnerAdapterError::UnsafePathDirectory(directory.clone()))?
            .is_dir()
        {
            return Err(TestRunnerAdapterError::UnsafePathDirectory(
                directory.clone(),
            ));
        }
        if !validated.contains(&canonical) {
            validated.push(canonical);
        }
    }
    Ok(validated)
}

fn validate_directory(
    path: &Path,
    error: TestRunnerAdapterError,
) -> Result<PathBuf, TestRunnerAdapterError> {
    if !path.is_absolute() {
        return Err(error);
    }
    let link_metadata = fs::symlink_metadata(path).map_err(|_| error.clone())?;
    if link_metadata.file_type().is_symlink() || !link_metadata.is_dir() {
        return Err(error);
    }
    let canonical = fs::canonicalize(path).map_err(|_| error.clone())?;
    if canonical != path {
        return Err(error);
    }
    Ok(canonical)
}

pub(crate) fn discover_executable(
    directories: &[PathBuf],
    name: &str,
) -> Result<Option<PathBuf>, String> {
    for directory in directories {
        let candidate = directory.join(name);
        match fs::symlink_metadata(&candidate) {
            Ok(_) => {
                safe_executable_metadata(&candidate).map_err(|error| error.to_string())?;
                let canonical = fs::canonicalize(&candidate).map_err(|error| error.to_string())?;
                if canonical != candidate {
                    return Err(format!(
                        "Testing executable is not an exact canonical path: {}",
                        candidate.display()
                    ));
                }
                return Ok(Some(canonical));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Testing executable inspection failed for {}: {error}",
                    candidate.display()
                ));
            }
        }
    }
    Ok(None)
}

fn safe_executable_metadata(path: &Path) -> Result<fs::Metadata, TestRunnerAdapterError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| TestRunnerAdapterError::UnsafeExecutable(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(TestRunnerAdapterError::UnsafeExecutable(path.into()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(TestRunnerAdapterError::UnsafeExecutable(path.into()));
        }
    }
    Ok(metadata)
}
