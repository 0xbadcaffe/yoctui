async fn run_lsblk(executable: &Path, timeout: Duration) -> Result<Vec<u8>, WicAdapterError> {
    let mut command = Command::new(executable);
    command
        .args([
            "--json",
            "--bytes",
            "--paths",
            "--output",
            "PATH,TYPE,MAJ:MIN,SIZE,MODEL,SERIAL,TRAN,RM,RO,MOUNTPOINTS",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = spawn_async_command(&mut command)
        .await
        .map_err(|error| WicAdapterError::DeviceDiscovery(error.to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| WicAdapterError::DeviceDiscovery("lsblk stdout is unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| WicAdapterError::DeviceDiscovery("lsblk stderr is unavailable".into()))?;
    let collect = async move {
        let mut stdout_bytes = Vec::new();
        let mut stderr_bytes = Vec::new();
        let mut bounded_stdout = stdout.take(MAX_WIC_DEVICE_JSON_BYTES + 1);
        let mut bounded_stderr = stderr.take(MAX_WIC_DEVICE_JSON_BYTES + 1);
        let stdout_read = bounded_stdout.read_to_end(&mut stdout_bytes);
        let stderr_read = bounded_stderr.read_to_end(&mut stderr_bytes);
        let (stdout_result, stderr_result, status) =
            tokio::join!(stdout_read, stderr_read, child.wait());
        stdout_result.map_err(|error| WicAdapterError::DeviceDiscovery(error.to_string()))?;
        stderr_result.map_err(|error| WicAdapterError::DeviceDiscovery(error.to_string()))?;
        let status = status.map_err(|error| WicAdapterError::DeviceDiscovery(error.to_string()))?;
        Ok::<_, WicAdapterError>((stdout_bytes, stderr_bytes, status))
    };
    let (stdout, stderr, status) = tokio::time::timeout(timeout, collect)
        .await
        .map_err(|_| WicAdapterError::DeviceDiscovery("lsblk timed out".into()))??;
    if stdout.len() as u64 > MAX_WIC_DEVICE_JSON_BYTES
        || stderr.len() as u64 > MAX_WIC_DEVICE_JSON_BYTES
    {
        return Err(WicAdapterError::DeviceDiscovery(
            "lsblk output exceeded its safety bound".into(),
        ));
    }
    if !status.success() {
        return Err(WicAdapterError::DeviceDiscovery(
            String::from_utf8_lossy(&stderr).trim().to_owned(),
        ));
    }
    Ok(stdout)
}

fn json_string(value: &serde_json::Value, field: &str) -> Result<String, WicAdapterError> {
    value
        .as_str()
        .filter(|value| !value.is_empty() && !value.chars().any(char::is_control))
        .map(str::to_owned)
        .ok_or_else(|| WicAdapterError::DeviceDiscovery(format!("invalid lsblk {field}")))
}

fn json_bounded_string(
    value: &serde_json::Value,
    field: &str,
    max_bytes: usize,
) -> Result<String, WicAdapterError> {
    let value = json_string(value, field)?;
    if value.len() > max_bytes {
        return Err(WicAdapterError::DeviceDiscovery(format!(
            "lsblk {field} exceeded its safety bound"
        )));
    }
    Ok(value)
}

fn json_optional_string(
    value: &serde_json::Value,
    field: &str,
) -> Result<Option<String>, WicAdapterError> {
    if value.is_null() {
        return Ok(None);
    }
    json_bounded_string(value, field, 256).map(Some)
}

fn json_u64(value: &serde_json::Value, field: &str) -> Result<u64, WicAdapterError> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
        .ok_or_else(|| WicAdapterError::DeviceDiscovery(format!("invalid lsblk {field}")))
}

fn json_bool(value: &serde_json::Value, field: &str) -> Result<bool, WicAdapterError> {
    value
        .as_bool()
        .or_else(|| {
            value.as_u64().and_then(|value| match value {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            })
        })
        .or_else(|| {
            value.as_str().and_then(|value| match value {
                "0" | "false" => Some(false),
                "1" | "true" => Some(true),
                _ => None,
            })
        })
        .ok_or_else(|| WicAdapterError::DeviceDiscovery(format!("invalid lsblk {field}")))
}

fn mountpoints(value: &serde_json::Value) -> Result<Vec<PathBuf>, WicAdapterError> {
    let values: Vec<&serde_json::Value> = match value {
        serde_json::Value::Null => Vec::new(),
        serde_json::Value::String(_) => vec![value],
        serde_json::Value::Array(values) => values.iter().collect(),
        _ => {
            return Err(WicAdapterError::DeviceDiscovery(
                "invalid lsblk mountpoints".into(),
            ));
        }
    };
    let mut paths = Vec::new();
    for value in values {
        if value.is_null() {
            continue;
        }
        let path = PathBuf::from(json_bounded_string(
            value,
            "mountpoint",
            MAX_WIC_DEVICE_PATH_BYTES,
        )?);
        if !path.is_absolute() {
            return Err(WicAdapterError::DeviceDiscovery(
                "lsblk returned a relative mountpoint".into(),
            ));
        }
        paths.push(path);
    }
    paths.sort();
    paths.dedup();
    if paths.len() > MAX_WIC_DEVICE_MOUNTS {
        return Err(WicAdapterError::DeviceDiscovery(
            "lsblk returned too many mountpoints".into(),
        ));
    }
    Ok(paths)
}

fn subtree_mounts(node: &LsblkNode, output: &mut Vec<PathBuf>) -> Result<(), WicAdapterError> {
    output.extend(mountpoints(&node.mountpoints)?);
    for child in &node.children {
        subtree_mounts(child, output)?;
    }
    Ok(())
}

fn subtree_has_root_mount(node: &LsblkNode) -> Result<bool, WicAdapterError> {
    if mountpoints(&node.mountpoints)?
        .iter()
        .any(|mount| mount == Path::new("/"))
    {
        return Ok(true);
    }
    for child in &node.children {
        if subtree_has_root_mount(child)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn validate_lsblk_records(
    nodes: &[LsblkNode],
    seen_paths: &mut BTreeMap<PathBuf, String>,
    seen_major_minor: &mut BTreeMap<String, PathBuf>,
    count: &mut usize,
) -> Result<(), WicAdapterError> {
    for node in nodes {
        *count = count.saturating_add(1);
        if *count > MAX_WIC_DEVICE_RECORDS {
            return Err(WicAdapterError::DeviceDiscovery(
                "lsblk returned too many device records".into(),
            ));
        }
        let path = PathBuf::from(json_bounded_string(
            &node.path,
            "path",
            MAX_WIC_DEVICE_PATH_BYTES,
        )?);
        let _ = json_bounded_string(&node.kind, "type", 32)?;
        let major_minor = json_bounded_string(&node.major_minor, "major:minor", 32)?;
        let _ = json_u64(&node.size, "size")?;
        let _ = json_optional_string(&node.model, "model")?;
        let _ = json_optional_string(&node.serial, "serial")?;
        let _ = json_optional_string(&node.tran, "transport")?;
        let _ = json_bool(&node.rm, "removable")?;
        let _ = json_bool(&node.ro, "read-only")?;
        let _ = mountpoints(&node.mountpoints)?;
        WicDeviceIdentity {
            path: path.clone(),
            major_minor: major_minor.clone(),
            size_bytes: 0,
            model: None,
            serial: None,
            transport: None,
        }
        .validate()
        .map_err(|message| WicAdapterError::DeviceDiscovery(message.into()))?;
        if seen_paths
            .insert(path.clone(), major_minor.clone())
            .is_some()
            || seen_major_minor.insert(major_minor, path).is_some()
        {
            return Err(WicAdapterError::DeviceDiscovery(
                "lsblk returned duplicate device identities".into(),
            ));
        }
        validate_lsblk_records(&node.children, seen_paths, seen_major_minor, count)?;
    }
    Ok(())
}

fn validate_device_node(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        let Ok(metadata) = fs::symlink_metadata(path) else {
            return false;
        };
        !metadata.file_type().is_symlink()
            && metadata.file_type().is_block_device()
            && fs::canonicalize(path).is_ok_and(|canonical| canonical == path)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}

fn device_is_writable(
    path: &Path,
    validate_device_nodes: bool,
    unwritable_device_nodes: &BTreeSet<PathBuf>,
) -> bool {
    !unwritable_device_nodes.contains(path)
        && (!validate_device_nodes || fs::OpenOptions::new().write(true).open(path).is_ok())
}

fn parse_lsblk_devices(
    bytes: &[u8],
    image: &WicOutputIdentity,
    validate_device_nodes: bool,
    unwritable_device_nodes: &BTreeSet<PathBuf>,
) -> Result<(Vec<WicDevice>, Vec<String>), WicAdapterError> {
    if bytes.len() as u64 > MAX_WIC_DEVICE_JSON_BYTES {
        return Err(WicAdapterError::DeviceDiscovery(
            "lsblk output exceeded its safety bound".into(),
        ));
    }
    let document: LsblkDocument = serde_json::from_slice(bytes)
        .map_err(|error| WicAdapterError::DeviceDiscovery(error.to_string()))?;
    let mut seen_paths = BTreeMap::<PathBuf, String>::new();
    let mut seen_major_minor = BTreeMap::<String, PathBuf>::new();
    let mut record_count = 0;
    validate_lsblk_records(
        &document.blockdevices,
        &mut seen_paths,
        &mut seen_major_minor,
        &mut record_count,
    )?;
    let mut root_devices = Vec::new();
    for node in &document.blockdevices {
        if json_bounded_string(&node.kind, "type", 32)? == "disk" && subtree_has_root_mount(node)? {
            root_devices.push(json_bounded_string(
                &node.path,
                "path",
                MAX_WIC_DEVICE_PATH_BYTES,
            )?);
        }
    }
    root_devices.sort();
    root_devices.dedup();
    if root_devices.len() != 1 {
        return Err(WicAdapterError::DeviceDiscovery(
            "the root backing whole device could not be identified uniquely".into(),
        ));
    }
    let root_device = &root_devices[0];
    let mut devices = Vec::new();
    let mut limitations = Vec::new();
    for node in &document.blockdevices {
        let path = PathBuf::from(json_bounded_string(
            &node.path,
            "path",
            MAX_WIC_DEVICE_PATH_BYTES,
        )?);
        let kind = json_bounded_string(&node.kind, "type", 32)?;
        let major_minor = json_bounded_string(&node.major_minor, "major:minor", 32)?;
        if kind != "disk" {
            limitations.push(format!("{}: excluded device type {kind}", path.display()));
            continue;
        }
        let size_bytes = json_u64(&node.size, "size")?;
        let removable = json_bool(&node.rm, "removable")?;
        let read_only = json_bool(&node.ro, "read-only")?;
        let mut descendant_mounts = Vec::new();
        subtree_mounts(node, &mut descendant_mounts)?;
        descendant_mounts.sort();
        descendant_mounts.dedup();
        let reason = if path.to_str() == Some(root_device) {
            Some("backs the current root filesystem")
        } else if !path.is_absolute() || !path.starts_with("/dev") {
            Some("path is not an absolute /dev identity")
        } else if validate_device_nodes && !validate_device_node(&path) {
            Some("path is not a canonical whole block-device node")
        } else if !removable {
            Some("device is not removable")
        } else if read_only {
            Some("device is read-only")
        } else if !descendant_mounts.is_empty() {
            Some("device has mounted descendants")
        } else if size_bytes < image.size_bytes {
            Some("device is smaller than the selected image")
        } else if !device_is_writable(&path, validate_device_nodes, unwritable_device_nodes) {
            Some("device cannot be opened for writing")
        } else {
            None
        };
        if let Some(reason) = reason {
            limitations.push(format!("{}: {reason}", path.display()));
            continue;
        }
        devices.push(WicDevice {
            identity: WicDeviceIdentity {
                path,
                major_minor,
                size_bytes,
                model: json_optional_string(&node.model, "model")?,
                serial: json_optional_string(&node.serial, "serial")?,
                transport: json_optional_string(&node.tran, "transport")?,
            },
            removable,
            writable: true,
            read_only,
            descendant_mounts,
            unavailable_reason: None,
        });
    }
    if devices.len() > MAX_WIC_DEVICES {
        return Err(WicAdapterError::DeviceDiscovery(
            "lsblk returned too many eligible devices".into(),
        ));
    }
    if limitations.len() > MAX_WIC_LIMITATIONS {
        let omitted = limitations.len() - (MAX_WIC_LIMITATIONS - 1);
        limitations.truncate(MAX_WIC_LIMITATIONS - 1);
        limitations.push(format!(
            "{omitted} additional unsafe device exclusions were omitted"
        ));
    }
    Ok((
        normalize_wic_devices(devices),
        normalize_wic_limitations(limitations),
    ))
}

fn validate_wic_image(image: &WicOutputIdentity) -> Result<(), WicAdapterError> {
    image
        .validate()
        .map_err(|_| WicAdapterError::UnsafeImage(image.path.clone()))?;
    let canonical = regular_canonical(&image.path)
        .map_err(|_| WicAdapterError::UnsafeImage(image.path.clone()))?;
    let size = fs::metadata(&canonical)
        .map_err(|_| WicAdapterError::UnsafeImage(image.path.clone()))?
        .len();
    if canonical != image.path || size != image.size_bytes {
        return Err(WicAdapterError::UnsafeImage(image.path.clone()));
    }
    Ok(())
}

fn resolve_executable(program: &Path) -> Result<Option<PathBuf>, String> {
    if program.is_absolute() {
        return if program.exists() {
            regular_executable(program)
                .map(Some)
                .map_err(|error| error.to_string())
        } else {
            Ok(None)
        };
    }
    if program.components().count() != 1
        || !matches!(program.components().next(), Some(Component::Normal(_)))
    {
        return Err("relative Wic executable candidates are ambiguous".into());
    }
    let Some(path) = std::env::var_os("PATH") else {
        return Ok(None);
    };
    for directory in std::env::split_paths(&path).filter(|path| path.is_absolute()) {
        let candidate = directory.join(program);
        if candidate.exists() {
            return regular_executable(&candidate)
                .map(Some)
                .map_err(|error| error.to_string());
        }
    }
    Ok(None)
}

fn regular_executable(path: &Path) -> Result<PathBuf, WicAdapterError> {
    let canonical =
        regular_canonical(path).map_err(|_| WicAdapterError::UnsafeExecutable(path.into()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(&canonical)
            .map_err(|_| WicAdapterError::UnsafeExecutable(path.into()))?
            .permissions()
            .mode()
            & 0o111
            == 0
        {
            return Err(WicAdapterError::UnsafeExecutable(path.into()));
        }
    }
    Ok(canonical)
}

fn regular_canonical(path: &Path) -> Result<PathBuf, ()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    if !path.is_absolute() || metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(());
    }
    let canonical = fs::canonicalize(path).map_err(|_| ())?;
    (canonical == path).then_some(canonical).ok_or(())
}

fn canonical_directory(path: &Path) -> Result<PathBuf, WicAdapterError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| WicAdapterError::UnsafeOutputDirectory(path.into()))?;
    let canonical =
        fs::canonicalize(path).map_err(|_| WicAdapterError::UnsafeOutputDirectory(path.into()))?;
    if !path.is_absolute()
        || metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || canonical != path
    {
        return Err(WicAdapterError::UnsafeOutputDirectory(path.into()));
    }
    Ok(canonical)
}
