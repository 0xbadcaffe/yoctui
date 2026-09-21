fn collect_result_files(roots: &[PathBuf]) -> Result<Vec<PathBuf>, TestResultAdapterError> {
    let mut files = BTreeSet::new();
    let mut directories_seen = 0usize;
    let mut entries_seen = 0usize;
    for root in roots {
        let metadata = fs::symlink_metadata(root)
            .map_err(|_| TestResultAdapterError::UnsafeRoot(root.clone()))?;
        if metadata.file_type().is_symlink() {
            return Err(TestResultAdapterError::UnsafeRoot(root.clone()));
        }
        let canonical =
            fs::canonicalize(root).map_err(|_| TestResultAdapterError::UnsafeRoot(root.clone()))?;
        if canonical != *root {
            return Err(TestResultAdapterError::UnsafeRoot(root.clone()));
        }
        if metadata.is_file() {
            if root.file_name().and_then(|value| value.to_str()) != Some("testresults.json") {
                return Err(TestResultAdapterError::UnsafeResult(root.clone()));
            }
            files.insert(root.clone());
            continue;
        }
        if !metadata.is_dir() {
            return Err(TestResultAdapterError::UnsafeRoot(root.clone()));
        }
        let mut pending = vec![root.clone()];
        while let Some(directory) = pending.pop() {
            directories_seen += 1;
            if directories_seen > MAX_RESULT_SCAN_DIRECTORIES {
                return Err(TestResultAdapterError::Bound(
                    "too many result directories".into(),
                ));
            }
            let mut entries = fs::read_dir(&directory)
                .map_err(|_| TestResultAdapterError::UnsafeRoot(directory.clone()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| TestResultAdapterError::UnsafeRoot(directory.clone()))?;
            entries.sort_by_key(fs::DirEntry::file_name);
            for entry in entries {
                entries_seen += 1;
                if entries_seen > MAX_RESULT_SCAN_ENTRIES {
                    return Err(TestResultAdapterError::Bound(
                        "too many result directory entries".into(),
                    ));
                }
                let path = entry.path();
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|_| TestResultAdapterError::UnsafeRoot(path.clone()))?;
                if metadata.file_type().is_symlink() {
                    continue;
                }
                if metadata.is_dir() {
                    pending.push(path);
                } else if metadata.is_file()
                    && path.file_name().and_then(|value| value.to_str()) == Some("testresults.json")
                {
                    files.insert(path);
                    if files.len() > MAX_RESULT_FILES {
                        return Err(TestResultAdapterError::Bound(
                            "too many testresults.json files".into(),
                        ));
                    }
                }
            }
        }
    }
    Ok(files.into_iter().collect())
}

fn parse_result_file(
    path: &Path,
) -> Result<(TestResultRecord, Vec<String>), TestResultAdapterError> {
    let metadata = safe_regular_file(path)?;
    if metadata.len() == 0 || metadata.len() > MAX_RESULT_FILE_BYTES {
        return Err(TestResultAdapterError::Bound(format!(
            "{} has an invalid byte size",
            path.display()
        )));
    }
    let bytes = fs::read(path).map_err(|_| TestResultAdapterError::UnsafeResult(path.into()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| TestResultAdapterError::Malformed(error.to_string()))?;
    let runs = value.as_object().ok_or_else(|| {
        TestResultAdapterError::Malformed("top-level result must be an object".into())
    })?;
    let identity = TestResultIdentity::new(
        path.into(),
        metadata.len(),
        metadata
            .modified()
            .map_err(|_| TestResultAdapterError::UnsafeResult(path.into()))?,
        fingerprint(&bytes),
    )
    .map_err(|_| TestResultAdapterError::UnsafeResult(path.into()))?;
    let mut suites = Vec::new();
    let mut limitations = Vec::new();
    let mut all_metadata = Vec::new();
    let mut family = None;
    let mut machine = None;
    let mut image = None;
    let mut revision = None;
    for (run_id, run) in runs {
        let Some(run) = run.as_object() else {
            limitations.push(format!("skipped malformed run {run_id}"));
            continue;
        };
        let configuration = run.get("configuration").and_then(Value::as_object);
        let cases = run.get("result").and_then(Value::as_object);
        let Some(cases) = cases else {
            limitations.push(format!("skipped run {run_id} without a result object"));
            continue;
        };
        if let Some(configuration) = configuration {
            family = family.or_else(|| family_from_configuration(configuration));
            machine = machine.or_else(|| configuration_string(configuration, "MACHINE"));
            image = image.or_else(|| {
                configuration_string(configuration, "IMAGE_BASENAME")
                    .or_else(|| configuration_string(configuration, "IMAGE_NAME"))
            });
            revision = revision.or_else(|| {
                configuration_string(configuration, "OECOREREV")
                    .or_else(|| configuration_string(configuration, "revision"))
            });
            all_metadata.extend(configuration_metadata(
                run_id,
                configuration,
                &mut limitations,
            ));
        }
        let mut typed_cases = Vec::new();
        for (case_name, case) in cases {
            match parse_case(path, run_id, case_name, case) {
                Ok((case, case_limitations)) => {
                    typed_cases.push(case);
                    limitations.extend(case_limitations);
                }
                Err(message) => {
                    limitations.push(format!(
                        "skipped malformed case {run_id}/{case_name}: {message}"
                    ));
                }
            }
        }
        let (suite, suite_limitations) =
            TestSuiteRecord::new(run_id.clone(), None, Vec::new(), typed_cases)
                .map_err(|message| TestResultAdapterError::Malformed(message.into()))?;
        suites.push(suite);
        limitations.extend(suite_limitations);
    }
    if suites.is_empty() {
        return Err(TestResultAdapterError::Malformed(
            "result file contains no typed result runs".into(),
        ));
    }
    let (record, _normalization) = TestResultRecord::new(
        identity,
        family,
        machine,
        image,
        revision,
        None,
        all_metadata,
        suites,
        None,
        limitations,
    );
    let response_limitations = record.limitations.clone();
    Ok((record, response_limitations))
}
