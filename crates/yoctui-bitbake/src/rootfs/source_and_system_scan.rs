fn scan_sources(
    request: RootfsCompositionRequest,
    build_directory: PathBuf,
    sources: RootfsCompositionSources,
    cancellation: RootfsCompositionCancellation,
    deadline: Instant,
) -> Result<RootfsCompositionResponse, RootfsCompositionAdapterError> {
    let build = canonical_directory(&build_directory, None)?;
    let mut limitations = Vec::new();
    let installed_packages = match sources.manifest {
        Some(manifest) if source_is_missing(&manifest)? => RootfsAuthority::Unavailable {
            reason: "the exact selected image manifest has been cleaned or is unavailable".into(),
        },
        Some(manifest) => scan_manifest(
            &build,
            &manifest,
            sources.pkgdata_directory.as_deref(),
            &cancellation,
            deadline,
            &mut limitations,
        )?,
        None => RootfsAuthority::Unavailable {
            reason: "the exact selected image manifest was not reported by BitBake".into(),
        },
    };
    let mut root_directory = None;
    let (filesystem_tree, system_inventory) = match sources.image_rootfs {
        Some(root) if source_is_missing(&root)? => (
            RootfsAuthority::Unavailable {
                reason: "the BitBake-reported IMAGE_ROOTFS has been cleaned or is unavailable"
                    .into(),
            },
            RootfsAuthority::Unavailable {
                reason: "offline system inventory requires IMAGE_ROOTFS".into(),
            },
        ),
        Some(root) => {
            let canonical_root = canonical_directory(&root, Some(&build))?;
            root_directory = Some(canonical_root.clone());
            (
                scan_filesystem(&build, &root, &cancellation, deadline, &mut limitations)?,
                scan_system_inventory(&canonical_root, &cancellation, deadline, &mut limitations)?,
            )
        }
        None => (
            RootfsAuthority::Unavailable {
                reason: "IMAGE_ROOTFS was not reported for the selected image".into(),
            },
            RootfsAuthority::Unavailable {
                reason: "offline system inventory requires IMAGE_ROOTFS".into(),
            },
        ),
    };
    limitations.sort();
    limitations.dedup();
    if limitations.len() > MAX_LIMITATIONS {
        limitations.truncate(MAX_LIMITATIONS - 1);
        limitations.push(format!(
            "rootfs limitation reporting was capped at {MAX_LIMITATIONS} records"
        ));
    }
    Ok(RootfsCompositionResponse {
        request: request.clone(),
        composition: RootfsComposition {
            image: request.image,
            installed_packages,
            filesystem_tree,
            system_inventory,
            root_directory,
        },
        limitations,
    })
}

fn scan_system_inventory(
    root: &Path,
    cancellation: &RootfsCompositionCancellation,
    deadline: Instant,
    limitations: &mut Vec<String>,
) -> Result<RootfsAuthority<RootfsSystemInventory>, RootfsCompositionAdapterError> {
    let mut local_limitations = Vec::new();
    let mut systemd_services = Vec::new();
    let unit_directories = [
        "etc/systemd/system",
        "run/systemd/system",
        "usr/local/lib/systemd/system",
        "usr/lib/systemd/system",
        "lib/systemd/system",
    ];
    let mut seen = BTreeSet::new();
    for relative_directory in unit_directories {
        check_control(cancellation, deadline)?;
        let directory = root.join(relative_directory);
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            if systemd_services.len() == MAX_SYSTEM_RECORDS {
                push_limitation(
                    &mut local_limitations,
                    format!("system service inventory was limited to {MAX_SYSTEM_RECORDS} records"),
                );
                break;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".service") || !seen.insert(name.clone()) {
                continue;
            }
            let host_path = entry.path();
            let Ok(canonical_host_path) = fs::canonicalize(&host_path) else {
                continue;
            };
            let Ok(metadata) = fs::metadata(&canonical_host_path) else {
                continue;
            };
            if !canonical_host_path.starts_with(root)
                || !metadata.is_file()
                || metadata.len() > MAX_SYSTEM_FILE_BYTES
            {
                continue;
            }
            let Ok(content) = fs::read_to_string(&host_path) else {
                continue;
            };
            let logical_path = RootfsPathIdentity(
                PathBuf::from("/").join(host_path.strip_prefix(root).unwrap_or(&host_path)),
            );
            let (preview, preview_truncated) = bounded_preview(&content);
            systemd_services.push(RootfsSystemdService {
                name: name.clone(),
                logical_path,
                host_path,
                description: ini_value(&content, "Description"),
                bus_name: ini_value(&content, "BusName"),
                enabled_by: systemd_enablement(root, &name),
                preview,
                preview_truncated,
            });
        }
    }

    let policy_files = collect_files(
        root,
        &[
            "etc/dbus-1/system.d",
            "usr/local/share/dbus-1/system.d",
            "usr/share/dbus-1/system.d",
            "usr/local/share/dbus-1/service.d",
            "usr/share/dbus-1/service.d",
        ],
        &[".conf"],
    );
    let mut dbus_services = Vec::new();
    let mut dbus_names = BTreeSet::new();
    for host_path in collect_files(
        root,
        &[
            "etc/dbus-1/system-services",
            "run/dbus-1/system-services",
            "usr/local/share/dbus-1/system-services",
            "usr/share/dbus-1/system-services",
            "lib/dbus-1/system-services",
        ],
        &[".service"],
    ) {
        check_control(cancellation, deadline)?;
        if dbus_services.len() == MAX_SYSTEM_RECORDS {
            push_limitation(
                &mut local_limitations,
                format!("system D-Bus inventory was limited to {MAX_SYSTEM_RECORDS} records"),
            );
            break;
        }
        let Ok(content) = read_bounded_text(&host_path) else {
            continue;
        };
        let Some(name) = ini_value(&content, "Name") else {
            continue;
        };
        if !dbus_names.insert(name.clone()) {
            continue;
        }
        let matching_policies = policy_files
            .iter()
            .filter(|path| {
                read_bounded_text(path).is_ok_and(|content| policy_mentions_name(&content, &name))
            })
            .map(|path| {
                RootfsPathIdentity(PathBuf::from("/").join(path.strip_prefix(root).unwrap_or(path)))
            })
            .collect();
        let (preview, preview_truncated) = bounded_preview(&content);
        dbus_services.push(RootfsDbusService {
            name,
            logical_path: RootfsPathIdentity(
                PathBuf::from("/").join(host_path.strip_prefix(root).unwrap_or(&host_path)),
            ),
            host_path,
            exec: ini_value(&content, "Exec"),
            user: ini_value(&content, "User"),
            systemd_service: ini_value(&content, "SystemdService"),
            policy_files: matching_policies,
            preview,
            preview_truncated,
        });
    }
    for service in &systemd_services {
        let Some(name) = service.bus_name.clone() else {
            continue;
        };
        if !dbus_names.insert(name.clone()) {
            continue;
        }
        let matching_policies = policy_files
            .iter()
            .filter(|path| {
                read_bounded_text(path).is_ok_and(|content| policy_mentions_name(&content, &name))
            })
            .map(|path| {
                RootfsPathIdentity(PathBuf::from("/").join(path.strip_prefix(root).unwrap_or(path)))
            })
            .collect();
        dbus_services.push(RootfsDbusService {
            name,
            logical_path: service.logical_path.clone(),
            host_path: service.host_path.clone(),
            exec: None,
            user: None,
            systemd_service: Some(service.name.clone()),
            policy_files: matching_policies,
            preview: service.preview.clone(),
            preview_truncated: service.preview_truncated,
        });
    }
    systemd_services.sort_by(|left, right| left.name.cmp(&right.name));
    dbus_services.sort_by(|left, right| left.name.cmp(&right.name));
    let udev_rules = udev::scan(root, cancellation, deadline, &mut local_limitations)?;
    limitations.extend(local_limitations.iter().cloned());
    let inventory = RootfsSystemInventory {
        systemd_services,
        dbus_services,
        udev_rules,
    };
    if local_limitations.is_empty() {
        Ok(RootfsAuthority::Available(inventory))
    } else {
        Ok(RootfsAuthority::Partial {
            value: inventory,
            limitations: local_limitations,
        })
    }
}

fn read_bounded_text(path: &Path) -> Result<String, ()> {
    let metadata = fs::metadata(path).map_err(|_| ())?;
    if !metadata.is_file() || metadata.len() > MAX_SYSTEM_FILE_BYTES {
        return Err(());
    }
    fs::read_to_string(path).map_err(|_| ())
}

fn ini_value(content: &str, key: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let line = line.trim();
        let (candidate, value) = line.split_once('=')?;
        (candidate.trim() == key)
            .then(|| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    })
}

fn bounded_preview(content: &str) -> (String, bool) {
    (
        yoctui_utils::utf8_prefix(content, MAX_ROOTFS_SYSTEM_PREVIEW_BYTES).to_owned(),
        content.len() > MAX_ROOTFS_SYSTEM_PREVIEW_BYTES,
    )
}

fn policy_mentions_name(content: &str, name: &str) -> bool {
    content.contains(&format!("\"{name}\"")) || content.contains(&format!("'{name}'"))
}

fn collect_files(root: &Path, directories: &[&str], suffixes: &[&str]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for relative in directories {
        let Ok(entries) = fs::read_dir(root.join(relative)) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let contained_file = fs::canonicalize(&path).is_ok_and(|canonical| {
                canonical.starts_with(root)
                    && fs::metadata(canonical).is_ok_and(|meta| meta.is_file())
            });
            if contained_file
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| suffixes.iter().any(|suffix| name.ends_with(suffix)))
            {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn systemd_enablement(root: &Path, service: &str) -> Vec<String> {
    let mut enabled_by = Vec::new();
    for base in ["etc/systemd/system", "run/systemd/system"] {
        let Ok(entries) = fs::read_dir(root.join(base)) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let directory = entry.path();
            let directory_name = entry.file_name().to_string_lossy().into_owned();
            if !(directory_name.ends_with(".wants") || directory_name.ends_with(".requires")) {
                continue;
            }
            if fs::symlink_metadata(directory.join(service)).is_ok() {
                enabled_by.push(directory_name);
            }
        }
    }
    enabled_by.sort();
    enabled_by
}
