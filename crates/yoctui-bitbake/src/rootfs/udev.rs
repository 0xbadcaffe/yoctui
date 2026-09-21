//! Offline file selection only: never evaluate rules or open host devices.
use super::*;
use std::{collections::BTreeMap, io::Read, path::Component};
use yoctui_model::{MAX_ROOTFS_UDEV_RULES, RootfsUdevRule};

// Highest priority first; /lib is the legacy vendor location.
const DIRECTORIES: [&str; 5] = [
    "etc/udev/rules.d",
    "run/udev/rules.d",
    "usr/local/lib/udev/rules.d",
    "usr/lib/udev/rules.d",
    "lib/udev/rules.d",
];

/// Resolve image-absolute links relative to IMAGE_ROOTFS, never the host /.
/// None denotes the image /dev/null mask without opening the device.
fn resolve(root: &Path, relative: &Path, depth: usize) -> Result<Option<PathBuf>, String> {
    if depth >= 32 {
        return Err("symlink resolution limit".into());
    }
    let mut normalized = PathBuf::new();
    for part in relative.components() {
        match part {
            Component::Normal(name) => normalized.push(name),
            Component::CurDir => {}
            Component::ParentDir if normalized.pop() => {}
            _ => return Err("path escapes IMAGE_ROOTFS".into()),
        }
    }
    if normalized == Path::new("dev/null") {
        return Ok(None);
    }
    let parts = normalized.components().collect::<Vec<_>>();
    let mut path = root.to_path_buf();
    for (index, part) in parts.iter().enumerate() {
        path.push(part.as_os_str());
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() {
            let target = fs::read_link(&path).map_err(|error| error.to_string())?;
            let mut next = if target.is_absolute() {
                target
                    .strip_prefix("/")
                    .map_err(|error| error.to_string())?
                    .to_path_buf()
            } else {
                path.parent()
                    .and_then(|parent| parent.strip_prefix(root).ok())
                    .ok_or("path escapes IMAGE_ROOTFS")?
                    .join(target)
            };
            for remainder in &parts[index + 1..] {
                next.push(remainder.as_os_str());
            }
            return resolve(root, &next, depth + 1);
        }
    }
    Ok(Some(path))
}

pub(super) fn scan(
    root: &Path,
    cancellation: &RootfsCompositionCancellation,
    deadline: Instant,
    limitations: &mut Vec<String>,
) -> Result<Vec<RootfsUdevRule>, RootfsCompositionAdapterError> {
    let mut rules = Vec::new();
    let mut winners = BTreeMap::new();
    let mut directories = BTreeSet::new();
    let mut visited = 0;
    for relative in DIRECTORIES {
        check_control(cancellation, deadline)?;
        let directory = match resolve(root, Path::new(relative), 0) {
            Ok(Some(path)) => path,
            result => {
                // Missing optional search paths are normal; existing broken links aren't.
                if fs::symlink_metadata(root.join(relative)).is_ok() {
                    push_limitation(
                        limitations,
                        format!("udev directory /{relative}: {result:?}"),
                    );
                }
                continue;
            }
        };
        if !directories.insert(directory.clone()) {
            continue;
        }
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                push_limitation(limitations, format!("udev directory /{relative}: {error}"));
                continue;
            }
        };
        for entry in entries {
            check_control(cancellation, deadline)?;
            visited += 1;
            if visited > MAX_ROOTFS_ENTRIES || rules.len() >= MAX_ROOTFS_UDEV_RULES {
                push_limitation(
                    limitations,
                    "udev inventory reached its entry/record safety bound".into(),
                );
                rules.sort();
                return Ok(rules);
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    push_limitation(limitations, format!("udev entry: {error}"));
                    continue;
                }
            };
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                push_limitation(limitations, "udev rule filename is not UTF-8".into());
                continue;
            };
            if !name.ends_with(".rules") {
                continue;
            }
            let logical_path = RootfsPathIdentity(Path::new("/").join(relative).join(name));
            let overridden_by = winners.get(name).cloned();
            winners
                .entry(name.to_owned())
                .or_insert_with(|| logical_path.clone());
            let mut rule = RootfsUdevRule {
                name: name.into(),
                logical_path,
                masked: false,
                overridden_by,
                limitation: None,
                preview: String::new(),
                preview_truncated: false,
            };
            match resolve(root, &Path::new(relative).join(name), 0) {
                Ok(None) => rule.masked = true,
                Ok(Some(path)) => match preview(&path) {
                    Ok((text, truncated)) => {
                        rule.preview = text;
                        rule.preview_truncated = truncated;
                    }
                    Err(reason) => rule.limitation = Some(reason),
                },
                Err(reason) => rule.limitation = Some(reason),
            }
            if let Some(reason) = &rule.limitation {
                push_limitation(limitations, format!("udev /{relative}/{name}: {reason}"));
            }
            rules.push(rule);
        }
    }
    rules.sort();
    Ok(rules)
}

fn preview(path: &Path) -> Result<(String, bool), String> {
    if !fs::symlink_metadata(path)
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("not a regular rule file".into());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|error| error.to_string())?;
    if !file
        .metadata()
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("not a regular rule file".into());
    }
    let mut bytes = Vec::new();
    file.take((MAX_ROOTFS_SYSTEM_PREVIEW_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let truncated = bytes.len() > MAX_ROOTFS_SYSTEM_PREVIEW_BYTES;
    bytes.truncate(MAX_ROOTFS_SYSTEM_PREVIEW_BYTES);
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    while text.len() > MAX_ROOTFS_SYSTEM_PREVIEW_BYTES {
        text.pop();
    }
    Ok((text, truncated))
}

#[cfg(test)]
#[path = "../tests/rootfs_udev/mod.rs"]
mod tests;
