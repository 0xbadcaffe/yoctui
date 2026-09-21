#[derive(Debug, Clone, PartialEq, Eq)]
struct SdkEnvironmentSetupIdentity {
    path: PathBuf,
    size_bytes: u64,
    modified_unix_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkToolCommandSpec {
    operation: SdkOperation,
    executable: PathBuf,
    arguments: Vec<OsString>,
    current_directory: PathBuf,
    environment: BTreeMap<OsString, OsString>,
    clear_environment: bool,
    environment_setup: Option<SdkEnvironmentSetupIdentity>,
    allowed_roots: Vec<PathBuf>,
    sdk_deploy_root: Option<PathBuf>,
}

impl SdkToolCommandSpec {
    pub fn operation(&self) -> &SdkOperation {
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

    pub fn environment(&self) -> &BTreeMap<OsString, OsString> {
        &self.environment
    }

    pub fn clears_environment(&self) -> bool {
        self.clear_environment
    }

    fn revalidate(&self) -> Result<(), SdkToolAdapterError> {
        let expected_name = match &self.operation {
            SdkOperation::Publish(_) => "oe-publish-sdk",
            SdkOperation::Native(request) => match request.mode {
                SdkNativeMode::FindSysroot => "oe-find-native-sysroot",
                SdkNativeMode::RunNative => "oe-run-native",
            },
        };
        validate_named_tool(&self.executable, expected_name, &self.allowed_roots)?;
        match &self.operation {
            SdkOperation::Publish(request) => {
                let Some(root) = &self.sdk_deploy_root else {
                    return Err(SdkToolAdapterError::PreviewMismatch);
                };
                validate_installer(&request.artifact, root)?;
                validate_empty_destination(&request.destination)?;
                if self.current_directory != request.destination
                    || self.arguments
                        != [
                            request.artifact.path.as_os_str().to_owned(),
                            request.destination.as_os_str().to_owned(),
                        ]
                {
                    return Err(SdkToolAdapterError::PreviewMismatch);
                }
            }
            SdkOperation::Native(request) => {
                let expected = native_arguments(request);
                if self.arguments != expected {
                    return Err(SdkToolAdapterError::PreviewMismatch);
                }
                if let Some(root) = &request.extracted_root {
                    let canonical = validate_exact_directory(
                        root,
                        SdkToolAdapterError::UnsafeExtractedRoot(root.clone()),
                    )?;
                    if self.current_directory != canonical || !self.clear_environment {
                        return Err(SdkToolAdapterError::PreviewMismatch);
                    }
                    let setup = find_environment_setup(&canonical)?;
                    let environment = parse_environment_setup(&setup.path)?;
                    if self.environment_setup.as_ref() != Some(&setup)
                        || self.environment != environment
                    {
                        return Err(SdkToolAdapterError::InvalidEnvironment(
                            "environment setup changed after preview".into(),
                        ));
                    }
                } else {
                    validate_exact_directory(
                        &self.current_directory,
                        SdkToolAdapterError::UnsafeBuildDirectory(self.current_directory.clone()),
                    )?;
                    if self.clear_environment
                        || self.environment_setup.is_some()
                        || !self.environment.is_empty()
                    {
                        return Err(SdkToolAdapterError::PreviewMismatch);
                    }
                }
            }
        }
        Ok(())
    }
}
