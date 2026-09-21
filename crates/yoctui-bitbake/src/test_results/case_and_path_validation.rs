fn parse_case(
    result_path: &Path,
    suite: &str,
    name: &str,
    value: &Value,
) -> Result<(TestCaseRecord, Vec<String>), String> {
    let case = value
        .as_object()
        .ok_or_else(|| "case value is not an object".to_string())?;
    let identity = TestCaseIdentity::new(suite.into(), name.into()).map_err(str::to_owned)?;
    let outcome = case
        .get("status")
        .and_then(Value::as_str)
        .map(|status| match status.to_ascii_uppercase().as_str() {
            "PASSED" | "PASS" => TestCaseOutcome::Passed,
            "FAILED" | "FAIL" | "EXPECTEDFAIL" => TestCaseOutcome::Failed,
            "SKIPPED" | "SKIP" => TestCaseOutcome::Skipped,
            "ERROR" => TestCaseOutcome::Error,
            _ => TestCaseOutcome::Unknown,
        })
        .unwrap_or(TestCaseOutcome::Unknown);
    let duration = case
        .get("duration")
        .and_then(Value::as_f64)
        .filter(|duration| duration.is_finite() && *duration >= 0.0)
        .and_then(|duration| Duration::try_from_secs_f64(duration).ok());
    let mut limitations = Vec::new();
    let log_path = case
        .get("log_path")
        .and_then(Value::as_str)
        .and_then(|value| {
            let path = PathBuf::from(value);
            match safe_regular_file(&path) {
                Ok(_) if path.is_absolute() => Some(path),
                _ => {
                    limitations.push(format!(
                        "ignored unsafe related log for {}",
                        result_path.display()
                    ));
                    None
                }
            }
        });
    let metadata = case
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "status" | "duration" | "log_path"))
        .filter_map(|(key, value)| {
            scalar_string(value)
                .and_then(|value| TestMetadata::new(key.clone(), value).map_err(|_| ()).ok())
        })
        .collect();
    TestCaseRecord::new(identity, outcome, duration, metadata, log_path)
        .map_err(str::to_owned)
        .map(|(record, normalized)| {
            limitations.extend(normalized);
            (record, limitations)
        })
}

fn configuration_metadata(
    run_id: &str,
    configuration: &Map<String, Value>,
    limitations: &mut Vec<String>,
) -> Vec<TestMetadata> {
    configuration
        .iter()
        .filter_map(|(key, value)| {
            let value = scalar_string(value)?;
            match TestMetadata::new(format!("{run_id}.{key}"), value) {
                Ok(metadata) => Some(metadata),
                Err(_) => {
                    limitations.push(format!(
                        "ignored oversized configuration field {run_id}.{key}"
                    ));
                    None
                }
            }
        })
        .collect()
}

fn family_from_configuration(configuration: &Map<String, Value>) -> Option<TestFamily> {
    let test_type = configuration_string(configuration, "TEST_TYPE")?.to_ascii_lowercase();
    match test_type.as_str() {
        "runtime" | "testimage" => Some(TestFamily::TestImage),
        "sdk" | "testsdk" => Some(TestFamily::TestSdk),
        "sdkext" | "testsdkext" => Some(TestFamily::TestSdkExt),
        "ptest" => Some(TestFamily::Ptest),
        "oeselftest" | "oe-selftest" | "selftest" => Some(TestFamily::OeSelftest),
        "bitbake-selftest" | "bitbakeselftest" => Some(TestFamily::BitbakeSelftest),
        _ => None,
    }
}

fn configuration_string(configuration: &Map<String, Value>, key: &str) -> Option<String> {
    configuration.get(key).and_then(scalar_string)
}

fn scalar_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

fn safe_regular_file(path: &Path) -> Result<fs::Metadata, TestResultAdapterError> {
    if !path.is_absolute() {
        return Err(TestResultAdapterError::UnsafeResult(path.into()));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| TestResultAdapterError::UnsafeResult(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(TestResultAdapterError::UnsafeResult(path.into()));
    }
    let canonical =
        fs::canonicalize(path).map_err(|_| TestResultAdapterError::UnsafeResult(path.into()))?;
    if canonical != path {
        return Err(TestResultAdapterError::UnsafeResult(path.into()));
    }
    Ok(metadata)
}

fn validate_result_identity(identity: &TestResultIdentity) -> Result<(), TestResultAdapterError> {
    if !identity.is_valid() {
        return Err(TestResultAdapterError::StaleResult(identity.path.clone()));
    }
    let metadata = safe_regular_file(&identity.path)
        .map_err(|_| TestResultAdapterError::StaleResult(identity.path.clone()))?;
    let bytes = fs::read(&identity.path)
        .map_err(|_| TestResultAdapterError::StaleResult(identity.path.clone()))?;
    if metadata.len() != identity.byte_size
        || metadata.modified().ok() != Some(identity.modified_at)
        || fingerprint(&bytes) != identity.fingerprint
    {
        return Err(TestResultAdapterError::StaleResult(identity.path.clone()));
    }
    Ok(())
}

fn validate_junit_destination(destination: &Path) -> Result<PathBuf, TestResultAdapterError> {
    if !destination.is_absolute()
        || destination.extension().and_then(|value| value.to_str()) != Some("xml")
    {
        return Err(TestResultAdapterError::UnsafeDestination(
            destination.into(),
        ));
    }
    match fs::symlink_metadata(destination) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        _ => {
            return Err(TestResultAdapterError::UnsafeDestination(
                destination.into(),
            ));
        }
    }
    let parent = destination
        .parent()
        .ok_or_else(|| TestResultAdapterError::UnsafeDestination(destination.into()))?;
    let metadata = fs::symlink_metadata(parent)
        .map_err(|_| TestResultAdapterError::UnsafeDestination(destination.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(TestResultAdapterError::UnsafeDestination(
            destination.into(),
        ));
    }
    let canonical = fs::canonicalize(parent)
        .map_err(|_| TestResultAdapterError::UnsafeDestination(destination.into()))?;
    if canonical != parent {
        return Err(TestResultAdapterError::UnsafeDestination(
            destination.into(),
        ));
    }
    Ok(canonical)
}

fn fingerprint(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    true
}
