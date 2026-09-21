fn validate_request(request: &SecurityReportRequest) -> Result<(), SecurityReportAdapterError> {
    let normalized = SecurityReportRequest::new(request.generation, request.paths.clone())
        .map_err(|message| SecurityReportAdapterError::InvalidRequest(message.into()))?;
    if &normalized != request {
        return Err(SecurityReportAdapterError::InvalidRequest(
            "paths must be sorted and unique".into(),
        ));
    }
    Ok(())
}

fn scan_reports(
    request: SecurityReportRequest,
    cancellation: SecurityReportCancellation,
    deadline: Instant,
) -> Result<SecurityReportResponse, SecurityReportAdapterError> {
    let roots = request
        .paths
        .iter()
        .map(|path| validate_explicit_path(path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut files = BTreeSet::new();
    let mut limitations = Vec::new();
    let mut failures = ScanFailures::default();
    let mut directory_count = 0_usize;

    for root in roots {
        check_control(&cancellation, deadline)?;
        let metadata = fs::symlink_metadata(&root).map_err(|error| path_error(&root, error))?;
        if metadata.is_file() {
            files.insert(root);
        } else {
            collect_directory_files(
                &root,
                &root,
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
    for path in files {
        check_control(&cancellation, deadline)?;
        if reports.len() >= MAX_SECURITY_REPORTS {
            push_limitation(
                &mut limitations,
                format!(
                    "Security report {} was omitted at the {MAX_SECURITY_REPORTS}-report bound",
                    path.display()
                ),
            );
            continue;
        }
        let before = fs::symlink_metadata(&path).map_err(|error| path_error(&path, error))?;
        if before.file_type().is_symlink() || !before.is_file() {
            push_limitation(
                &mut limitations,
                format!(
                    "Security report became unsafe before parsing: {}",
                    path.display()
                ),
            );
            continue;
        }
        let size = before.len();
        if size == 0 {
            failures.malformed.get_or_insert_with(|| path.clone());
            push_limitation(
                &mut limitations,
                format!("empty Security report was ignored: {}", path.display()),
            );
            continue;
        }
        if size > MAX_SECURITY_FILE_BYTES {
            failures.oversized.get_or_insert_with(|| path.clone());
            push_limitation(
                &mut limitations,
                format!(
                    "Security report exceeded the {MAX_SECURITY_FILE_BYTES}-byte per-file bound: {}",
                    path.display()
                ),
            );
            continue;
        }
        if total_bytes.saturating_add(size) > MAX_SECURITY_TOTAL_BYTES {
            failures.oversized.get_or_insert_with(|| path.clone());
            push_limitation(
                &mut limitations,
                format!(
                    "Security report was omitted at the {MAX_SECURITY_TOTAL_BYTES}-byte total bound: {}",
                    path.display()
                ),
            );
            continue;
        }
        let Some(modified_at) = before.modified().ok() else {
            push_limitation(
                &mut limitations,
                format!(
                    "Security report modification time was unavailable: {}",
                    path.display()
                ),
            );
            continue;
        };
        let bytes = read_bounded_file(&path, size, &cancellation, deadline)?;
        total_bytes = total_bytes.saturating_add(bytes.len() as u64);
        let after = fs::symlink_metadata(&path).map_err(|error| path_error(&path, error))?;
        if after.file_type().is_symlink()
            || !after.is_file()
            || after.len() != size
            || after.modified().ok() != Some(modified_at)
            || fs::canonicalize(&path).ok().as_ref() != Some(&path)
        {
            failures.stale.get_or_insert_with(|| path.clone());
            push_limitation(
                &mut limitations,
                format!(
                    "Security report changed while it was acquired and was ignored: {}",
                    path.display()
                ),
            );
            continue;
        }
        let fingerprint = format!("{:x}", Sha256::digest(&bytes));
        let identity = SecurityReportIdentity::new(path.clone(), size, modified_at, fingerprint)
            .map_err(|message| SecurityReportAdapterError::Io(message.into()))?;
        match parse_report(identity, &bytes, &mut limitations, &cancellation, deadline)? {
            ParseReportOutcome::Report(report) => reports.push(*report),
            ParseReportOutcome::Malformed => {
                failures.malformed.get_or_insert_with(|| path.clone());
                push_limitation(
                    &mut limitations,
                    format!("malformed Security report was ignored: {}", path.display()),
                );
            }
            ParseReportOutcome::Unsupported => {
                failures.unsupported.get_or_insert_with(|| path.clone());
                push_limitation(
                    &mut limitations,
                    format!(
                        "unsupported Security report was ignored: {}",
                        path.display()
                    ),
                );
            }
        }
    }

    let (reports, model_limitations) = normalize_security_reports(reports);
    for limitation in model_limitations {
        push_limitation(&mut limitations, limitation);
    }
    for report in &reports {
        let (path, report_limitations) = match report {
            SecurityReport::Cve(report) => (&report.identity.path, &report.limitations),
            SecurityReport::Spdx(report) => (&report.identity.path, &report.limitations),
            SecurityReport::CycloneDx(report) => (&report.identity.path, &report.limitations),
            SecurityReport::PackageManifest(report) => (&report.identity.path, &report.limitations),
        };
        for limitation in report_limitations {
            push_limitation(
                &mut limitations,
                format!("{}: {limitation}", path.display()),
            );
        }
    }
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_SECURITY_LIMITATIONS);
    if reports.is_empty() && !limitations.is_empty() {
        return Err(failures.terminal_error(&limitations));
    }
    let outcome = if reports.is_empty() {
        SecurityReportScanOutcome::Empty
    } else if limitations.is_empty() {
        SecurityReportScanOutcome::Complete(reports)
    } else {
        SecurityReportScanOutcome::Partial {
            reports,
            limitations,
        }
    };
    Ok(SecurityReportResponse { request, outcome })
}

#[allow(clippy::too_many_arguments)]
fn collect_directory_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeSet<PathBuf>,
    directory_count: &mut usize,
    limitations: &mut Vec<String>,
    failures: &mut ScanFailures,
    cancellation: &SecurityReportCancellation,
    deadline: Instant,
) -> Result<(), SecurityReportAdapterError> {
    if *directory_count >= MAX_SECURITY_DIRECTORIES {
        push_limitation(
            limitations,
            format!(
                "Security directory was omitted at the {MAX_SECURITY_DIRECTORIES}-directory bound: {}",
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
        let entry = match entry {
            Ok(entry) => entry.path(),
            Err(error) => {
                push_limitation(
                    limitations,
                    format!("a Security directory entry was unreadable: {error}"),
                );
                continue;
            }
        };
        entries.insert(entry);
        if entries.len() > MAX_SECURITY_DIRECTORY_ENTRIES {
            entries.pop_last();
            omitted += 1;
        }
    }
    if omitted > 0 {
        push_limitation(
            limitations,
            format!(
                "{omitted} Security entries were omitted from {} at the {MAX_SECURITY_DIRECTORY_ENTRIES}-entry bound",
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
                        "Security entry metadata was unavailable for {}: {error}",
                        path.display()
                    ),
                );
                continue;
            }
        };
        if metadata.file_type().is_symlink() {
            failures.symlink.get_or_insert_with(|| path.clone());
            push_limitation(
                limitations,
                format!("Security symlink was not followed: {}", path.display()),
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
                failures.escape.get_or_insert_with(|| path.clone());
                push_limitation(
                    limitations,
                    format!(
                        "Security entry escaped its explicit root or was not canonical: {}",
                        path.display()
                    ),
                );
                continue;
            }
        };
        if metadata.is_dir() {
            collect_directory_files(
                root,
                &canonical,
                files,
                directory_count,
                limitations,
                failures,
                cancellation,
                deadline,
            )?;
        } else if metadata.is_file() {
            if is_candidate(&canonical) {
                files.insert(canonical);
            } else {
                failures.unsupported.get_or_insert_with(|| path.clone());
                push_limitation(
                    limitations,
                    format!("unsupported Security entry was ignored: {}", path.display()),
                );
            }
        } else {
            push_limitation(
                limitations,
                format!("non-regular Security entry was ignored: {}", path.display()),
            );
        }
    }
    Ok(())
}

fn validate_explicit_path(path: &Path) -> Result<PathBuf, SecurityReportAdapterError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| path_error(path, error))?;
    if metadata.file_type().is_symlink() {
        return Err(SecurityReportAdapterError::SymlinkPath(path.into()));
    }
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(SecurityReportAdapterError::UnsafePath(path.into()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| SecurityReportAdapterError::Io(error.to_string()))?;
    if canonical != path || canonical == Path::new("/") {
        return Err(SecurityReportAdapterError::EscapePath(path.into()));
    }
    if metadata.is_file() && !is_candidate(&canonical) {
        return Err(SecurityReportAdapterError::UnsupportedPath(path.into()));
    }
    Ok(canonical)
}

fn path_error(path: &Path, error: io::Error) -> SecurityReportAdapterError {
    match error.kind() {
        io::ErrorKind::NotFound => SecurityReportAdapterError::MissingPath(path.into()),
        io::ErrorKind::PermissionDenied => {
            SecurityReportAdapterError::PermissionDenied(path.into())
        }
        _ => SecurityReportAdapterError::Io(format!("{}: {error}", path.display())),
    }
}

fn read_bounded_file(
    path: &Path,
    expected_size: u64,
    cancellation: &SecurityReportCancellation,
    deadline: Instant,
) -> Result<Vec<u8>, SecurityReportAdapterError> {
    let mut file = fs::File::open(path).map_err(|error| path_error(path, error))?;
    let mut bytes = Vec::with_capacity(expected_size as usize);
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_control(cancellation, deadline)?;
        let read = file
            .read(&mut buffer)
            .map_err(|error| SecurityReportAdapterError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if bytes.len().saturating_add(read) > MAX_SECURITY_FILE_BYTES as usize {
            return Err(SecurityReportAdapterError::Io(format!(
                "Security report grew beyond its bound: {}",
                path.display()
            )));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn is_candidate(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    name.ends_with(".json")
        || name.ends_with(".jsonl")
        || name.ends_with(".cve")
        || name.ends_with(".cve.txt")
        || name.ends_with(".cve.log")
        || name.ends_with(".spdx.tar.zst")
        || name.ends_with(".spdx.tar.gz")
        || name.ends_with(".spdx.zip")
        || name.ends_with(".manifest")
}

enum ParseReportOutcome {
    Report(Box<SecurityReport>),
    Malformed,
    Unsupported,
}
