//! Layer browser.
use super::*;

pub(crate) fn scan_layer_directory(
    scan: &Path,
    inspect_git: bool,
) -> io::Result<Vec<LayerBrowserEntry>> {
    let git_output = inspect_git
        .then(|| {
            ProcessCommand::new("git")
                .args([
                    "status",
                    "--porcelain=v1",
                    "-z",
                    "--no-renames",
                    "--ignored=matching",
                    "--untracked-files=normal",
                    "--",
                    ".",
                ])
                .current_dir(scan)
                .output()
                .ok()
                .filter(|output| output.status.success())
        })
        .flatten();
    let relative = git_output.and_then(|output| {
        let prefix = ProcessCommand::new("git")
            .args(["rev-parse", "--show-prefix"])
            .current_dir(scan)
            .output()
            .ok()
            .filter(|output| output.status.success())?;
        directory_relative_git_output(&prefix.stdout, &output.stdout)
    });
    let mut entries = scan_layer_directory_stream(scan, |_| true)?;
    decorate_entries(scan, &mut entries, relative.as_deref());
    Ok(entries)
}

pub(crate) fn directory_relative_git_output(prefix: &[u8], output: &[u8]) -> Option<Vec<u8>> {
    let prefix = std::str::from_utf8(prefix.strip_suffix(b"\n")?).ok()?;
    if Path::new(prefix).is_absolute()
        || Path::new(prefix)
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return None;
    }
    let output = std::str::from_utf8(output).ok()?;
    if !output.is_empty() && !output.ends_with('\0') {
        return None;
    }
    let mut relative = Vec::with_capacity(output.len());
    for row in output.split('\0').filter(|row| !row.is_empty()) {
        if row.len() < 4 || !row.as_bytes()[..2].is_ascii() || row.as_bytes()[2] != b' ' {
            return None;
        }
        let name = &row[3..];
        let name = if !prefix.is_empty() && name == prefix.trim_end_matches('/') {
            ""
        } else {
            name.strip_prefix(prefix)?
        };
        relative.extend_from_slice(&row.as_bytes()[..3]);
        relative.extend_from_slice(if name.is_empty() {
            b"."
        } else {
            name.as_bytes()
        });
        relative.push(0);
    }
    Some(relative)
}

pub(crate) fn scan_layer_directory_stream(
    scan: &Path,
    mut emit: impl FnMut(&[LayerBrowserEntry]) -> bool,
) -> io::Result<Vec<LayerBrowserEntry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(scan)? {
        if !emit(&entries) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "directory scan cancelled",
            ));
        }
        if entries.len() >= yoctui_model::MAX_ROOTFS_ENTRIES {
            return Err(io::Error::other("directory exceeds the entry bound"));
        }
        if let Ok(entry) = entry {
            let path = entry.path();
            let name = entry.file_name();
            if name == ".git" {
                continue;
            }
            let metadata = entry.metadata().ok();
            let is_dir = metadata.as_ref().is_some_and(|value| value.is_dir());
            entries.push(LayerBrowserEntry {
                path,
                is_dir,
                depth: 0,
                is_hidden: name.to_string_lossy().starts_with('.'),
                size: metadata.as_ref().map(|value| value.len()),
                modified: metadata.and_then(|value| value.modified().ok()),
                git: GitFileState::Unavailable,
                rootfs_metadata: None,
            });
        }
    }
    entries.sort_by_key(|entry| {
        (
            !entry.is_dir,
            entry.path.file_name().map(|name| name.to_owned()),
        )
    });
    Ok(entries)
}

pub(crate) fn decorate_entries(
    scan: &Path,
    entries: &mut [LayerBrowserEntry],
    output: Option<&[u8]>,
) {
    use std::collections::HashMap;
    let mut states = HashMap::new();
    let mut default = GitFileState::Clean;
    let rank = |state| match state {
        GitFileState::Modified => 3,
        GitFileState::Untracked => 2,
        GitFileState::Ignored => 1,
        _ => 0,
    };
    let parsed = output
        .filter(|bytes| bytes.is_empty() || bytes.last() == Some(&0))
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .filter(|text| {
            for row in text.split('\0').filter(|row| !row.is_empty()) {
                if row.len() < 4 || !row.as_bytes()[..2].is_ascii() || row.as_bytes()[2] != b' ' {
                    return false;
                }
                let Some(name) = row.get(3..) else {
                    return false;
                };
                let state = match &row[..2] {
                    "??" => GitFileState::Untracked,
                    "!!" => GitFileState::Ignored,
                    _ => GitFileState::Modified,
                };
                if name == "." {
                    if rank(state) > rank(default) {
                        default = state;
                    }
                    continue;
                }
                let relative = Path::new(name);
                if relative.is_absolute()
                    || relative
                        .components()
                        .any(|part| !matches!(part, std::path::Component::Normal(_)))
                {
                    return false;
                }
                let Some(first) = relative.components().next() else {
                    return false;
                };
                states
                    .entry(scan.join(first.as_os_str()))
                    .and_modify(|old| {
                        if rank(state) > rank(*old) {
                            *old = state;
                        }
                    })
                    .or_insert(state);
            }
            true
        });
    for entry in entries {
        entry.git = if parsed.is_some() {
            states.get(&entry.path).copied().unwrap_or(default)
        } else {
            GitFileState::Unavailable
        };
    }
}

pub(crate) fn read_layer_preview(path: &Path) -> io::Result<(String, PreviewKind, bool)> {
    const MAX_PREVIEW_BYTES: usize = 64 * 1024;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let mut file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Only regular files can be previewed",
        ));
    }
    let size = metadata.len();
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(MAX_PREVIEW_BYTES as u64)
        .read_to_end(&mut bytes)?;
    let truncated = size > bytes.len() as u64;
    if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
        Ok((String::new(), PreviewKind::Binary, truncated))
    } else {
        Ok((
            String::from_utf8(bytes).expect("UTF-8 was validated"),
            PreviewKind::Text,
            truncated,
        ))
    }
}
