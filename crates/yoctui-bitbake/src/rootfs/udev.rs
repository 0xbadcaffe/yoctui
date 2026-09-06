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
mod tests {
    use super::*;

    struct TestRoot(PathBuf);
    impl TestRoot {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("yoctui-udev-{}-{nonce}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn udev_inventory_preserves_overrides_masks_and_bounded_preview() {
        let temp = TestRoot::new();
        let root = temp.path();
        for dir in DIRECTORIES {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        fs::write(
            root.join("usr/lib/udev/rules.d/10-device.rules"),
            "SUBSYSTEM==\"tty\"\n",
        )
        .unwrap();
        fs::write(
            root.join("etc/udev/rules.d/10-device.rules"),
            "# override\n",
        )
        .unwrap();
        fs::write(
            root.join("usr/lib/udev/rules.d/20-large.rules"),
            "x".repeat(9000),
        )
        .unwrap();
        fs::write(root.join("etc/udev/rules.d/ignored.txt"), "ignored").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/dev/null", root.join("etc/udev/rules.d/30-mask.rules"))
            .unwrap();
        let mut limitations = Vec::new();
        let rules = scan(
            root,
            &RootfsCompositionCancellation::default(),
            Instant::now() + ROOTFS_SCAN_TIMEOUT,
            &mut limitations,
        )
        .unwrap();
        assert!(limitations.is_empty(), "{limitations:?}");
        assert_eq!(
            rules
                .iter()
                .filter(|rule| rule.name == "10-device.rules")
                .count(),
            2
        );
        assert!(rules.iter().any(|rule| rule.overridden_by.is_some()));
        assert!(
            rules
                .iter()
                .any(|rule| rule.preview_truncated && rule.preview.len() == 8192)
        );
        #[cfg(unix)]
        assert!(rules.iter().any(|rule| rule.status() == "Masked"));
    }

    #[cfg(unix)]
    #[test]
    fn udev_image_absolute_links_stay_in_image_and_loops_are_bounded() {
        let temp = TestRoot::new();
        fs::create_dir_all(temp.path().join("usr/lib/udev/rules.d")).unwrap();
        fs::write(
            temp.path().join("usr/lib/udev/rules.d/10-test.rules"),
            "# image only",
        )
        .unwrap();
        std::os::unix::fs::symlink("/usr/lib", temp.path().join("lib")).unwrap();
        assert_eq!(
            resolve(temp.path(), Path::new("lib/udev/rules.d/10-test.rules"), 0).unwrap(),
            Some(temp.path().join("usr/lib/udev/rules.d/10-test.rules"))
        );
        std::os::unix::fs::symlink("loop", temp.path().join("loop")).unwrap();
        assert!(resolve(temp.path(), Path::new("loop"), 0).is_err());
        assert!(resolve(temp.path(), Path::new("../outside"), 0).is_err());
    }

    #[test]
    fn udev_empty_scan_cancellation_and_deadline_are_explicit() {
        let root = TestRoot::new();
        let token = RootfsCompositionCancellation::default();
        let mut limitations = Vec::new();
        assert!(
            scan(
                root.path(),
                &token,
                Instant::now() + ROOTFS_SCAN_TIMEOUT,
                &mut limitations
            )
            .unwrap()
            .is_empty()
        );
        assert!(limitations.is_empty());
        assert!(matches!(
            scan(root.path(), &token, Instant::now(), &mut limitations),
            Err(RootfsCompositionAdapterError::Timeout(_))
        ));
        token.cancel();
        assert_eq!(
            scan(
                root.path(),
                &token,
                Instant::now() + ROOTFS_SCAN_TIMEOUT,
                &mut limitations
            ),
            Err(RootfsCompositionAdapterError::Cancelled)
        );
    }

    #[cfg(unix)]
    #[test]
    fn udev_unresolved_file_is_retained_and_limits_are_reported() {
        let root = TestRoot::new();
        let directory = root.path().join("etc/udev/rules.d");
        fs::create_dir_all(&directory).unwrap();
        std::os::unix::fs::symlink("/missing/image/file", directory.join("00-broken.rules"))
            .unwrap();
        let mut limitations = Vec::new();
        let token = RootfsCompositionCancellation::default();
        let rules = scan(
            root.path(),
            &token,
            Instant::now() + ROOTFS_SCAN_TIMEOUT,
            &mut limitations,
        )
        .unwrap();
        assert_eq!(rules[0].status(), "Unresolved");
        assert!(!limitations.is_empty());
        for index in 0..MAX_ROOTFS_UDEV_RULES {
            fs::write(directory.join(format!("{index:05}.rules")), "").unwrap();
        }
        let rules = scan(
            root.path(),
            &token,
            Instant::now() + ROOTFS_SCAN_TIMEOUT,
            &mut limitations,
        )
        .unwrap();
        assert_eq!(rules.len(), MAX_ROOTFS_UDEV_RULES);
        assert!(
            limitations
                .iter()
                .any(|reason| reason.contains("safety bound"))
        );
    }
}
