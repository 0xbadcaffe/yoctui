//! Read-only, contained acquisition for the on-disk RootFS file browser.
use std::{
    collections::HashMap,
    fs,
    io::{self, Read},
    path::Path,
};
use yoctui_model::{LayerBrowserEntry, PreviewKind, RootfsEntryKind, RootfsFileMetadata};

fn contained(root: &Path, path: &Path) -> io::Result<()> {
    if !root.is_absolute()
        || fs::canonicalize(root)? != root
        || !path.starts_with(root)
        || fs::canonicalize(path)? != path
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "RootFS path is outside the canonical root or traverses a symlink",
        ));
    }
    Ok(())
}

fn host_accounts(path: &Path) -> HashMap<u32, String> {
    let mut bytes = Vec::new();
    let Ok(file) = fs::File::open(path) else {
        return HashMap::new();
    };
    if file.take(1024 * 1024 + 1).read_to_end(&mut bytes).is_err() || bytes.len() > 1024 * 1024 {
        return HashMap::new();
    }
    String::from_utf8_lossy(&bytes)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(':');
            let name = fields.next()?;
            fields.next()?;
            let id = fields.next()?.parse().ok()?;
            Some((id, name.to_owned()))
        })
        .collect()
}

pub fn scan_rootfs_browser_directory(
    root: &Path,
    directory: &Path,
) -> io::Result<Vec<LayerBrowserEntry>> {
    contained(root, directory)?;
    let owners = host_accounts(Path::new("/etc/passwd"));
    let groups = host_accounts(Path::new("/etc/group"));
    let mut entries = Vec::new();
    for child in fs::read_dir(directory)? {
        if entries.len() == yoctui_model::MAX_ROOTFS_ENTRIES {
            return Err(io::Error::other("RootFS directory exceeds the entry bound"));
        }
        let child = child?;
        let path = child.path();
        let metadata = fs::symlink_metadata(&path).ok();
        let is_dir = metadata.as_ref().is_some_and(|value| value.is_dir());
        #[cfg(unix)]
        let attributes = metadata.as_ref().map(|value| {
            use std::os::unix::fs::MetadataExt;
            RootfsFileMetadata {
                kind: if value.is_dir() {
                    RootfsEntryKind::Directory
                } else if value.is_file() {
                    RootfsEntryKind::RegularFile
                } else if value.file_type().is_symlink() {
                    RootfsEntryKind::Symlink
                } else {
                    RootfsEntryKind::Other
                },
                mode: value.mode(),
                uid: value.uid(),
                gid: value.gid(),
                owner: owners.get(&value.uid()).cloned(),
                group: groups.get(&value.gid()).cloned(),
                link_target: value
                    .file_type()
                    .is_symlink()
                    .then(|| fs::read_link(&path).ok())
                    .flatten(),
            }
        });
        #[cfg(not(unix))]
        let attributes = {
            let _ = (&owners, &groups);
            None
        };
        entries.push(LayerBrowserEntry {
            path,
            is_dir,
            is_hidden: child.file_name().to_string_lossy().starts_with('.'),
            size: metadata.as_ref().map(|value| value.len()),
            modified: metadata.and_then(|value| value.modified().ok()),
            rootfs_metadata: attributes,
            ..Default::default()
        });
    }
    entries.sort_by_key(|entry| {
        (
            !entry.is_dir,
            entry.path.file_name().map(|name| name.to_owned()),
        )
    });
    Ok(entries)
}

pub fn read_rootfs_browser_preview(
    root: &Path,
    path: &Path,
) -> io::Result<(String, PreviewKind, bool)> {
    contained(root, path)?;
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Only regular RootFS files can be previewed",
        ));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::other("RootFS file changed type before preview"));
    }
    const LIMIT: usize = 64 * 1024;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(LIMIT as u64)
        .read_to_end(&mut bytes)?;
    let truncated = metadata.len() > bytes.len() as u64;
    if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
        Ok((String::new(), PreviewKind::Binary, truncated))
    } else {
        Ok((
            String::from_utf8(bytes).expect("validated UTF-8"),
            PreviewKind::Text,
            truncated,
        ))
    }
}

#[cfg(test)]
#[path = "tests/rootfs_browser.rs"]
mod tests;
