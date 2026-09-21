fn validate_input(input: &QaReportScanInput) -> Result<(), QaReportAdapterError> {
    let normalized = QaReportRequest::new(input.request.generation, input.request.paths.clone())
        .map_err(|message| QaReportAdapterError::InvalidRequest(message.into()))?;
    if normalized != input.request {
        return Err(QaReportAdapterError::InvalidRequest(
            "request paths must be sorted and unique".into(),
        ));
    }
    let build = validate_directory(&input.build_directory)?;
    if build != input.build_directory {
        return Err(QaReportAdapterError::InvalidRequest(
            "build directory must be canonical".into(),
        ));
    }
    if input.candidates.is_empty() || input.candidates.len() > input.request.paths.len() {
        return Err(QaReportAdapterError::InvalidRequest(
            "candidate count does not match the exact request".into(),
        ));
    }
    let mut candidate_paths = input
        .candidates
        .iter()
        .map(|candidate| candidate.path.clone())
        .collect::<Vec<_>>();
    candidate_paths.sort();
    if candidate_paths != input.request.paths {
        return Err(QaReportAdapterError::InvalidRequest(
            "candidate paths do not match the exact request".into(),
        ));
    }
    let mut unique = BTreeSet::new();
    for candidate in &input.candidates {
        if !unique.insert(candidate.path.clone())
            || !input.known_checks.contains(&candidate.producer)
            || !input.known_scopes.contains(&candidate.scope)
            || !candidate.producer.is_valid()
            || !candidate.scope.is_valid()
            || candidate
                .task
                .as_deref()
                .is_some_and(|value| !bounded_token(value))
            || candidate
                .test_name
                .as_deref()
                .is_some_and(|value| !bounded_text(value))
            || !matches!(
                (&candidate.scope, &candidate.task, &candidate.test_name),
                (QaFindingScope::Recipe(_), _, None) | (QaFindingScope::Layer(_), None, Some(_))
            )
        {
            return Err(QaReportAdapterError::InvalidRequest(
                "candidate scope, producer, task, or identity is invalid".into(),
            ));
        }
    }
    Ok(())
}

fn scan_reports(
    input: QaReportScanInput,
    cancellation: QaReportCancellation,
    deadline: Instant,
) -> Result<QaReportResponse, QaReportAdapterError> {
    let build_directory = validate_directory(&input.build_directory)?;
    let mut files = BTreeMap::<PathBuf, ExactFile>::new();
    let mut limitations = Vec::new();
    let mut failures = ScanFailures::default();
    let mut directory_count = 0_usize;

    for candidate in &input.candidates {
        check_control(&cancellation, deadline)?;
        let root = validate_explicit_path(&candidate.path)?;
        if candidate.origin == QaReportOrigin::Managed && !root.starts_with(&build_directory) {
            return Err(QaReportAdapterError::EscapePath(root));
        }
        let metadata = fs::symlink_metadata(&root).map_err(|error| path_error(&root, error))?;
        if metadata.is_file() {
            let Some(format) = candidate.format else {
                return Err(QaReportAdapterError::InvalidRequest(
                    "an exact file candidate must supply its documented format".into(),
                ));
            };
            if documented_format(&root) != Some(format) {
                return Err(QaReportAdapterError::UnsupportedPath(root));
            }
            files.insert(root.clone(), exact_file(candidate, root, format));
        } else {
            if candidate.format.is_some() {
                return Err(QaReportAdapterError::InvalidRequest(
                    "a directory candidate must not force one file format".into(),
                ));
            }
            collect_directory_files(
                &root,
                &root,
                candidate,
                &mut files,
                &mut directory_count,
                &mut limitations,
                &mut failures,
                &cancellation,
                deadline,
            )?;
        }
    }

    let mut reports = Vec::new();
    let mut total_bytes = 0_u64;
    for file in files.into_values() {
        check_control(&cancellation, deadline)?;
        if reports.len() >= MAX_QA_REPORTS {
            push_limitation(
                &mut limitations,
                format!(
                    "QA report {} was omitted at the {MAX_QA_REPORTS}-report bound",
                    file.path.display()
                ),
            );
            continue;
        }
        let before = validate_regular_file(&file.path)?;
        let size = before.len();
        if size == 0 {
            failures.malformed.get_or_insert_with(|| file.path.clone());
            push_limitation(
                &mut limitations,
                format!("empty QA report was ignored: {}", file.path.display()),
            );
            continue;
        }
        if size > MAX_QA_FILE_BYTES || total_bytes.saturating_add(size) > MAX_QA_TOTAL_BYTES {
            failures.oversized.get_or_insert_with(|| file.path.clone());
            push_limitation(
                &mut limitations,
                format!("oversized QA report was ignored: {}", file.path.display()),
            );
            continue;
        }
        let Some(modified_at) = before.modified().ok() else {
            failures
                .unsafe_path
                .get_or_insert_with(|| file.path.clone());
            push_limitation(
                &mut limitations,
                format!(
                    "QA report modification time was unavailable: {}",
                    file.path.display()
                ),
            );
            continue;
        };
        let bytes = read_bounded_file(&file.path, size, &cancellation, deadline)?;
        total_bytes = total_bytes.saturating_add(bytes.len() as u64);
        let after = validate_regular_file(&file.path)?;
        if after.len() != size
            || after.modified().ok() != Some(modified_at)
            || fs::canonicalize(&file.path).ok().as_ref() != Some(&file.path)
        {
            failures.stale.get_or_insert_with(|| file.path.clone());
            push_limitation(
                &mut limitations,
                format!(
                    "QA report changed while it was acquired: {}",
                    file.path.display()
                ),
            );
            continue;
        }
        let identity = QaReportIdentity::new(
            file.path.clone(),
            size,
            modified_at,
            fingerprint(&bytes),
            file.format,
            Some(file.producer.clone()),
            Some(file.scope.clone()),
        )
        .map_err(|message| QaReportAdapterError::Io(message.into()))?;
        match parse_report(identity, &file, &bytes, &mut limitations) {
            Ok(Some(report)) => reports.push(report),
            Ok(None) | Err(QaReportAdapterError::MalformedReport(_)) => {
                failures.malformed.get_or_insert_with(|| file.path.clone());
                push_limitation(
                    &mut limitations,
                    format!("malformed QA report was ignored: {}", file.path.display()),
                );
            }
            Err(error) => return Err(error),
        }
    }

    let (reports, model_limitations) =
        normalize_qa_reports(reports, &input.known_checks, &input.known_scopes);
    for limitation in model_limitations {
        push_limitation(&mut limitations, limitation);
    }
    for report in &reports {
        for limitation in &report.limitations {
            push_limitation(
                &mut limitations,
                format!("{}: {limitation}", report.identity.path.display()),
            );
        }
    }
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_QA_LIMITATIONS);
    if reports.is_empty() && !limitations.is_empty() {
        return Err(failures.terminal_error(&limitations));
    }
    let outcome = if reports.is_empty() {
        QaReportScanOutcome::Empty
    } else if limitations.is_empty() {
        QaReportScanOutcome::Complete(reports)
    } else {
        QaReportScanOutcome::Partial {
            reports,
            limitations,
        }
    };
    Ok(QaReportResponse {
        request: input.request,
        outcome,
    })
}

fn exact_file(candidate: &QaReportCandidate, path: PathBuf, format: QaReportFormat) -> ExactFile {
    ExactFile {
        path,
        format,
        producer: candidate.producer.clone(),
        scope: candidate.scope.clone(),
        task: candidate.task.clone(),
        test_name: candidate.test_name.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_directory_files(
    root: &Path,
    directory: &Path,
    candidate: &QaReportCandidate,
    files: &mut BTreeMap<PathBuf, ExactFile>,
    directory_count: &mut usize,
    limitations: &mut Vec<String>,
    failures: &mut ScanFailures,
    cancellation: &QaReportCancellation,
    deadline: Instant,
) -> Result<(), QaReportAdapterError> {
    if *directory_count >= MAX_QA_DIRECTORIES {
        push_limitation(
            limitations,
            format!(
                "QA directory was omitted at the {MAX_QA_DIRECTORIES}-directory bound: {}",
                directory.display()
            ),
        );
        return Ok(());
    }
    *directory_count += 1;
    let reader = fs::read_dir(directory).map_err(|error| path_error(directory, error))?;
    let mut entries = BTreeSet::new();
    let mut omitted = 0_usize;
    for entry in reader {
        check_control(cancellation, deadline)?;
        match entry {
            Ok(entry) => {
                entries.insert(entry.path());
                if entries.len() > MAX_QA_DIRECTORY_ENTRIES {
                    entries.pop_last();
                    omitted += 1;
                }
            }
            Err(error) => push_limitation(
                limitations,
                format!("a QA directory entry was unreadable: {error}"),
            ),
        }
    }
    if omitted > 0 {
        push_limitation(
            limitations,
            format!(
                "{omitted} QA entries were omitted from {} at the {MAX_QA_DIRECTORY_ENTRIES}-entry bound",
                directory.display()
            ),
        );
    }
    for path in entries {
        check_control(cancellation, deadline)?;
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                push_limitation(
                    limitations,
                    format!(
                        "QA entry metadata was unavailable for {}: {error}",
                        path.display()
                    ),
                );
                continue;
            }
        };
        if metadata.file_type().is_symlink() {
            failures.unsafe_path.get_or_insert_with(|| path.clone());
            push_limitation(
                limitations,
                format!("QA symlink was not followed: {}", path.display()),
            );
            continue;
        }
        let canonical = match fs::canonicalize(&path) {
            Ok(canonical)
                if canonical == path && canonical.starts_with(root) && canonical != root =>
            {
                canonical
            }
            _ => {
                failures.unsafe_path.get_or_insert_with(|| path.clone());
                push_limitation(
                    limitations,
                    format!("QA entry escaped its exact root: {}", path.display()),
                );
                continue;
            }
        };
        if metadata.is_dir() {
            collect_directory_files(
                root,
                &canonical,
                candidate,
                files,
                directory_count,
                limitations,
                failures,
                cancellation,
                deadline,
            )?;
        } else if metadata.is_file() {
            if let Some(format) = documented_format(&canonical) {
                if files.len() >= MAX_QA_REPORTS {
                    push_limitation(
                        limitations,
                        format!(
                            "QA report was omitted at the {MAX_QA_REPORTS}-file bound: {}",
                            canonical.display()
                        ),
                    );
                } else {
                    files.insert(canonical.clone(), exact_file(candidate, canonical, format));
                }
            } else {
                failures
                    .unsupported
                    .get_or_insert_with(|| canonical.clone());
                push_limitation(
                    limitations,
                    format!("unsupported QA entry was ignored: {}", canonical.display()),
                );
            }
        } else {
            failures
                .unsafe_path
                .get_or_insert_with(|| canonical.clone());
            push_limitation(
                limitations,
                format!("non-regular QA entry was ignored: {}", canonical.display()),
            );
        }
    }
    Ok(())
}

fn validate_explicit_path(path: &Path) -> Result<PathBuf, QaReportAdapterError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| path_error(path, error))?;
    if metadata.file_type().is_symlink() {
        return Err(QaReportAdapterError::SymlinkPath(path.into()));
    }
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(QaReportAdapterError::UnsafePath(path.into()));
    }
    let canonical =
        fs::canonicalize(path).map_err(|error| QaReportAdapterError::Io(error.to_string()))?;
    if canonical != path || canonical == Path::new("/") {
        return Err(QaReportAdapterError::EscapePath(path.into()));
    }
    Ok(canonical)
}

fn validate_directory(path: &Path) -> Result<PathBuf, QaReportAdapterError> {
    let canonical = validate_explicit_path(path)?;
    if !fs::symlink_metadata(&canonical)
        .map_err(|error| path_error(&canonical, error))?
        .is_dir()
    {
        return Err(QaReportAdapterError::UnsafePath(path.into()));
    }
    Ok(canonical)
}

fn validate_regular_file(path: &Path) -> Result<fs::Metadata, QaReportAdapterError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| path_error(path, error))?;
    if metadata.file_type().is_symlink() {
        return Err(QaReportAdapterError::SymlinkPath(path.into()));
    }
    if !metadata.is_file() || fs::canonicalize(path).ok().as_deref() != Some(path) {
        return Err(QaReportAdapterError::UnsafePath(path.into()));
    }
    Ok(metadata)
}

fn path_error(path: &Path, error: io::Error) -> QaReportAdapterError {
    match error.kind() {
        io::ErrorKind::NotFound => QaReportAdapterError::MissingPath(path.into()),
        io::ErrorKind::PermissionDenied => QaReportAdapterError::PermissionDenied(path.into()),
        _ => QaReportAdapterError::Io(format!("{}: {error}", path.display())),
    }
}

fn documented_format(path: &Path) -> Option<QaReportFormat> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    if name.ends_with(".json") || name.ends_with(".jsonl") {
        Some(QaReportFormat::Json)
    } else if name.ends_with(".xml") {
        Some(QaReportFormat::Xml)
    } else if name.ends_with(".qa") || name.ends_with(".txt") {
        Some(QaReportFormat::Text)
    } else if name.ends_with(".log") {
        Some(QaReportFormat::BitBakeLog)
    } else {
        None
    }
}

fn read_bounded_file(
    path: &Path,
    expected_size: u64,
    cancellation: &QaReportCancellation,
    deadline: Instant,
) -> Result<Vec<u8>, QaReportAdapterError> {
    if expected_size > MAX_QA_FILE_BYTES {
        return Err(QaReportAdapterError::OversizedReport(path.into()));
    }
    let mut file = fs::File::open(path).map_err(|error| path_error(path, error))?;
    let mut bytes = Vec::with_capacity(expected_size as usize);
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_control(cancellation, deadline)?;
        let read = file
            .read(&mut buffer)
            .map_err(|error| QaReportAdapterError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read) > MAX_QA_FILE_BYTES as usize {
            return Err(QaReportAdapterError::OversizedReport(path.into()));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn fingerprint(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn check_control(
    cancellation: &QaReportCancellation,
    deadline: Instant,
) -> Result<(), QaReportAdapterError> {
    if cancellation.is_cancelled() {
        Err(QaReportAdapterError::Cancelled)
    } else if Instant::now() >= deadline {
        Err(QaReportAdapterError::Timeout(0))
    } else {
        Ok(())
    }
}
