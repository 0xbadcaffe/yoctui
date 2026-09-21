fn populate_package(
    package: &mut RootfsInstalledPackage,
    values: &PkgdataValues,
    limitations: &mut Vec<String>,
) {
    package.recipe = values
        .recipe
        .as_ref()
        .filter(|value| valid_text(value))
        .cloned();
    package.category = values
        .category
        .as_ref()
        .filter(|value| valid_text(value) && !value.is_empty())
        .cloned()
        .or_else(|| package.recipe.clone())
        .unwrap_or_else(|| "uncategorized".into());
    match values.installed_size {
        Some(value) => package.installed_size_bytes = value,
        None => push_limitation(
            limitations,
            format!(
                "installed size was unavailable for package {}",
                package.identity.name
            ),
        ),
    }
    match (values.files_info_seen, values.file_count) {
        (_, Some(value)) => package.file_count = value,
        (true, None) => push_limitation(
            limitations,
            format!(
                "file count was malformed for package {}",
                package.identity.name
            ),
        ),
        (false, None) => push_limitation(
            limitations,
            format!(
                "file count was unavailable for package {}",
                package.identity.name
            ),
        ),
    }
}

fn scan_filesystem(
    build: &Path,
    image_rootfs: &Path,
    cancellation: &RootfsCompositionCancellation,
    deadline: Instant,
    limitations: &mut Vec<String>,
) -> Result<RootfsAuthority<RootfsFilesystemTree>, RootfsCompositionAdapterError> {
    check_control(cancellation, deadline)?;
    let root = canonical_directory(image_rootfs, Some(build))?;
    let mut entries = Vec::new();
    let mut stack = vec![(root.clone(), PathBuf::from("/"), 0_usize)];
    let mut local_limitations = Vec::new();
    let mut accounted_bytes = 0_u64;
    #[cfg(unix)]
    let mut hardlinks = BTreeSet::<(u64, u64)>::new();

    while let Some((host_path, logical_path, depth)) = stack.pop() {
        check_control(cancellation, deadline)?;
        if entries.len() == MAX_ROOTFS_ENTRIES {
            push_limitation(
                &mut local_limitations,
                format!("filesystem traversal was limited to {MAX_ROOTFS_ENTRIES} entries"),
            );
            break;
        }
        let metadata = fs::symlink_metadata(&host_path)
            .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
        let kind = classify_file_type(&metadata.file_type());
        let mut size = if matches!(kind, RootfsEntryKind::Directory) {
            0
        } else {
            metadata.len()
        };
        #[cfg(unix)]
        if matches!(kind, RootfsEntryKind::RegularFile)
            && metadata.nlink() > 1
            && !hardlinks.insert((metadata.dev(), metadata.ino()))
        {
            size = 0;
        }
        if accounted_bytes.saturating_add(size) > MAX_ROOTFS_ACCOUNTED_BYTES {
            push_limitation(
                &mut local_limitations,
                format!(
                    "filesystem byte accounting was limited to {MAX_ROOTFS_ACCOUNTED_BYTES} bytes"
                ),
            );
            size = 0;
        } else {
            accounted_bytes += size;
        }
        entries.push(RootfsEntry {
            identity: RootfsPathIdentity(logical_path.clone()),
            kind,
            size_bytes: size,
            package: None,
        });

        if matches!(kind, RootfsEntryKind::Directory) {
            if depth == MAX_ROOTFS_DEPTH {
                push_limitation(
                    &mut local_limitations,
                    format!("filesystem traversal was limited to depth {MAX_ROOTFS_DEPTH}"),
                );
                continue;
            }
            let mut children = fs::read_dir(&host_path)
                .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
            children.sort_by_key(fs::DirEntry::file_name);
            for child in children.into_iter().rev() {
                let file_name = child.file_name();
                let Some(name) = file_name.to_str() else {
                    push_limitation(
                        &mut local_limitations,
                        "one filesystem entry had a non-UTF-8 name".into(),
                    );
                    continue;
                };
                let mut child_logical = logical_path.clone();
                child_logical.push(name);
                stack.push((child.path(), child_logical, depth + 1));
            }
        }
    }
    if !entries.is_empty() {
        push_limitation(
            &mut local_limitations,
            "filesystem package ownership is unavailable from IMAGE_ROOTFS traversal".into(),
        );
    }
    limitations.extend(local_limitations.iter().cloned());
    let tree = RootfsFilesystemTree { entries };
    if local_limitations.is_empty() {
        Ok(RootfsAuthority::Available(tree))
    } else {
        Ok(RootfsAuthority::Partial {
            value: tree,
            limitations: local_limitations,
        })
    }
}

fn canonical_directory(
    path: &Path,
    containment_root: Option<&Path>,
) -> Result<PathBuf, RootfsCompositionAdapterError> {
    if !path.is_absolute() {
        return Err(RootfsCompositionAdapterError::InvalidSource(path.into()));
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        if containment_root.is_none() {
            RootfsCompositionAdapterError::BuildDirectory(path.into())
        } else {
            RootfsCompositionAdapterError::InvalidSource(path.into())
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(if containment_root.is_none() {
            RootfsCompositionAdapterError::BuildDirectory(path.into())
        } else {
            RootfsCompositionAdapterError::InvalidSource(path.into())
        });
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
    if containment_root.is_some_and(|root| !canonical.starts_with(root) || canonical == root) {
        return Err(RootfsCompositionAdapterError::PathEscape(canonical));
    }
    Ok(canonical)
}

fn canonical_regular_file(
    path: &Path,
    containment_root: &Path,
) -> Result<PathBuf, RootfsCompositionAdapterError> {
    if !path.is_absolute() {
        return Err(RootfsCompositionAdapterError::InvalidSource(path.into()));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| RootfsCompositionAdapterError::InvalidSource(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(RootfsCompositionAdapterError::InvalidSource(path.into()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
    if !canonical.starts_with(containment_root) || canonical == containment_root {
        return Err(RootfsCompositionAdapterError::PathEscape(canonical));
    }
    Ok(canonical)
}

fn source_is_missing(path: &Path) -> Result<bool, RootfsCompositionAdapterError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(RootfsCompositionAdapterError::Io(error.to_string())),
    }
}

fn classify_file_type(file_type: &fs::FileType) -> RootfsEntryKind {
    if file_type.is_dir() {
        RootfsEntryKind::Directory
    } else if file_type.is_file() {
        RootfsEntryKind::RegularFile
    } else if file_type.is_symlink() {
        RootfsEntryKind::Symlink
    } else {
        RootfsEntryKind::Other
    }
}

fn check_control(
    cancellation: &RootfsCompositionCancellation,
    deadline: Instant,
) -> Result<(), RootfsCompositionAdapterError> {
    if cancellation.is_cancelled() {
        Err(RootfsCompositionAdapterError::Cancelled)
    } else if Instant::now() >= deadline {
        Err(RootfsCompositionAdapterError::Timeout(0))
    } else {
        Ok(())
    }
}

fn push_limitation(limitations: &mut Vec<String>, limitation: String) {
    yoctui_utils::push_unique_bounded(limitations, limitation, MAX_LIMITATIONS);
}

fn valid_text(value: &str) -> bool {
    yoctui_utils::is_bounded_plain_text(value, 512)
}
