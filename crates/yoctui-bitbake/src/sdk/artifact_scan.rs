fn scan_deploy_root(
    request: SdkArtifactInventoryRequest,
    deploy_root: PathBuf,
    cancellation: SdkArtifactCancellation,
    deadline: Instant,
) -> Result<SdkArtifactResponse, SdkArtifactAdapterError> {
    validate_root(&deploy_root)?;
    let root =
        fs::canonicalize(&deploy_root).map_err(|error| root_io_error(&deploy_root, error))?;
    if root != deploy_root {
        return Err(SdkArtifactAdapterError::InvalidRoot(deploy_root));
    }
    check_scan_control(&cancellation, deadline)?;

    let mut limitations = Vec::new();
    let mut directories = BTreeSet::from([root.clone()]);
    let mut visited_directories = 0_usize;
    let mut files = Vec::new();
    let mut omitted_directories = 0_usize;
    let mut omitted_records = 0_usize;

    while let Some(directory) = directories.pop_first() {
        visited_directories = visited_directories.saturating_add(1);
        check_scan_control(&cancellation, deadline)?;
        let entries = bounded_directory_entries(
            &directory,
            directory == root,
            &cancellation,
            deadline,
            &mut limitations,
        )?;
        for path in entries {
            check_scan_control(&cancellation, deadline)?;
            if path.as_os_str().len() > MAX_SDK_PATH_BYTES {
                push_limitation(
                    &mut limitations,
                    format!("SDK entry exceeded the {MAX_SDK_PATH_BYTES}-byte path bound"),
                );
                continue;
            }
            if !valid_record_name(&path) {
                push_limitation(
                    &mut limitations,
                    "SDK entry had a malformed or oversized name".into(),
                );
                continue;
            }
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    push_limitation(
                        &mut limitations,
                        format!(
                            "metadata was unavailable for SDK entry {}: {error}",
                            path_label(&path)
                        ),
                    );
                    continue;
                }
            };
            if metadata.file_type().is_symlink() {
                push_limitation(
                    &mut limitations,
                    format!("SDK symlink was not followed: {}", path_label(&path)),
                );
                continue;
            }
            if metadata.is_dir() {
                if visited_directories.saturating_add(directories.len()) >= MAX_SDK_DIRECTORIES {
                    omitted_directories = omitted_directories.saturating_add(1);
                } else {
                    let canonical = canonical_descendant(&root, &path)?;
                    directories.insert(canonical);
                }
                continue;
            }
            if !metadata.is_file() {
                push_limitation(
                    &mut limitations,
                    format!("non-regular SDK entry was ignored: {}", path_label(&path)),
                );
                continue;
            }
            if files.len() >= MAX_SDK_ARTIFACTS {
                omitted_records = omitted_records.saturating_add(1);
                continue;
            }
            let canonical = canonical_descendant(&root, &path)?;
            let Some(kind) = classify_record(&canonical) else {
                push_limitation(
                    &mut limitations,
                    format!(
                        "malformed SDK record was ignored: {}",
                        path_label(&canonical)
                    ),
                );
                continue;
            };
            let Some(modified_unix_seconds) = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
            else {
                push_limitation(
                    &mut limitations,
                    format!(
                        "SDK record modification time was unavailable: {}",
                        path_label(&canonical)
                    ),
                );
                continue;
            };
            files.push(FileRecord {
                path: canonical,
                size_bytes: metadata.len(),
                modified_unix_seconds,
                kind,
            });
        }
    }

    if omitted_directories > 0 {
        push_limitation(
            &mut limitations,
            format!(
                "{omitted_directories} SDK directories were omitted at the {MAX_SDK_DIRECTORIES}-directory bound"
            ),
        );
    }
    if omitted_records > 0 {
        push_limitation(
            &mut limitations,
            format!(
                "{omitted_records} SDK records were omitted at the {MAX_SDK_ARTIFACTS}-record bound"
            ),
        );
    }

    files.sort_by(|left, right| left.path.cmp(&right.path));
    let mut artifacts = files
        .into_iter()
        .map(|record| SdkArtifact {
            identity: SdkArtifactIdentity {
                path: record.path,
                size_bytes: record.size_bytes,
                modified_unix_seconds: record.modified_unix_seconds,
            },
            kind: record.kind,
            sdk_kind: None,
            machine: None,
            host_tuple: None,
            target_tuple: None,
            checksums: Vec::new(),
            manifests: Vec::new(),
            published: None,
        })
        .collect::<Vec<_>>();
    associate_records(&mut artifacts, &mut limitations);
    let artifacts = normalize_sdk_artifacts(&request, artifacts)
        .map_err(|message| SdkArtifactAdapterError::Io(message.into()))?;
    let limitations = normalize_sdk_limitations(limitations);
    let outcome = if artifacts.is_empty() && limitations.is_empty() {
        SdkArtifactScanOutcome::Empty
    } else if limitations.is_empty() {
        SdkArtifactScanOutcome::Complete(artifacts)
    } else {
        SdkArtifactScanOutcome::Partial {
            artifacts,
            limitations,
        }
    };
    Ok(SdkArtifactResponse { request, outcome })
}

fn validate_root(root: &Path) -> Result<(), SdkArtifactAdapterError> {
    if !root.is_absolute() {
        return Err(SdkArtifactAdapterError::InvalidRoot(root.into()));
    }
    let metadata = fs::symlink_metadata(root).map_err(|error| root_io_error(root, error))?;
    if metadata.file_type().is_symlink() {
        return Err(SdkArtifactAdapterError::SymlinkRoot(root.into()));
    }
    if !metadata.is_dir() {
        return Err(SdkArtifactAdapterError::InvalidRoot(root.into()));
    }
    Ok(())
}

fn root_io_error(path: &Path, error: io::Error) -> SdkArtifactAdapterError {
    match error.kind() {
        io::ErrorKind::NotFound => SdkArtifactAdapterError::MissingRoot(path.into()),
        io::ErrorKind::PermissionDenied => SdkArtifactAdapterError::PermissionDenied(path.into()),
        _ => SdkArtifactAdapterError::Io(format!("{}: {error}", path.display())),
    }
}

fn bounded_directory_entries(
    directory: &Path,
    is_root: bool,
    cancellation: &SdkArtifactCancellation,
    deadline: Instant,
    limitations: &mut Vec<String>,
) -> Result<Vec<PathBuf>, SdkArtifactAdapterError> {
    let reader = match fs::read_dir(directory) {
        Ok(reader) => reader,
        Err(error) if is_root => return Err(root_io_error(directory, error)),
        Err(error) => {
            push_limitation(
                limitations,
                format!(
                    "SDK directory was unreadable and skipped: {}: {error}",
                    path_label(directory)
                ),
            );
            return Ok(Vec::new());
        }
    };
    let mut selected = BTreeSet::new();
    let mut omitted = 0_usize;
    for entry in reader {
        check_scan_control(cancellation, deadline)?;
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(error) => {
                push_limitation(
                    limitations,
                    format!("one SDK directory entry was unreadable: {error}"),
                );
                continue;
            }
        };
        selected.insert(path);
        if selected.len() > MAX_DIRECTORY_ENTRIES {
            selected.pop_last();
            omitted = omitted.saturating_add(1);
        }
    }
    if omitted > 0 {
        push_limitation(
            limitations,
            format!(
                "{omitted} SDK entries were omitted from {} at the {MAX_DIRECTORY_ENTRIES}-entry bound",
                path_label(directory)
            ),
        );
    }
    Ok(selected.into_iter().collect())
}
