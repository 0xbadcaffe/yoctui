fn scan_outputs(root: &Path) -> Result<WicOutputScan, WicAdapterError> {
    let root = canonical_directory(root)?;
    let mut files = BTreeMap::new();
    let mut limitations = Vec::new();
    for (index, entry) in fs::read_dir(&root)
        .map_err(|error| WicAdapterError::OutputScan(error.to_string()))?
        .enumerate()
    {
        if index >= MAX_WIC_OUTPUT_ENTRIES {
            limitations.push(format!(
                "Wic output scan was limited to {MAX_WIC_OUTPUT_ENTRIES} entries"
            ));
            break;
        }
        let Ok(entry) = entry else {
            limitations.push("one Wic output entry was unreadable".into());
            continue;
        };
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            limitations.push(format!("metadata unavailable for {}", path.display()));
            continue;
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            continue;
        }
        let Ok(canonical) = fs::canonicalize(&path) else {
            limitations.push(format!("could not canonicalize {}", path.display()));
            continue;
        };
        if canonical != path || !canonical.starts_with(&root) {
            limitations.push(format!("unsafe Wic output ignored: {}", path.display()));
            continue;
        }
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_nanos());
        files.insert(canonical, (metadata.len(), modified));
    }
    Ok((files, limitations))
}

fn classify_output(path: &Path) -> WicOutputKind {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if name.ends_with(".wic") {
        WicOutputKind::Wic
    } else if name.ends_with(".direct") {
        WicOutputKind::Direct
    } else if name.ends_with(".bmap") {
        WicOutputKind::Bmap
    } else if name.ends_with(".gz") || name.ends_with(".bz2") || name.ends_with(".xz") {
        WicOutputKind::Compressed
    } else {
        WicOutputKind::Other
    }
}
