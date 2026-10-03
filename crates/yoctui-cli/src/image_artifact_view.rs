//! Exact, bounded local artifact observations; never mount or extract an image.
use super::*;
use yoctui_model::{ImageArtifactView, PlatformFileKind, TEXTAREA_MAX_BYTES};

#[cfg(unix)]
fn open_image_file(root: &Path, path: &Path) -> Result<fs::File> {
    use std::os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::OpenOptionsExt},
    };
    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC)
        .open(root)?;
    let parts = path.strip_prefix(root)?.components().collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        let name = std::ffi::CString::new(part.as_os_str().as_bytes())?;
        let leaf = index + 1 == parts.len();
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | if leaf {
                libc::O_NONBLOCK
            } else {
                libc::O_DIRECTORY
            };
        // Owned directory descriptor anchors each relative lookup; no symlink
        // ancestor race can redirect the file open outside the deploy tree.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(io::Error::last_os_error().into());
        }
        // openat returned a new descriptor whose sole owner is this File.
        let opened = unsafe { fs::File::from_raw_fd(fd) };
        if leaf {
            return Ok(opened);
        }
        directory = opened;
    }
    anyhow::bail!("Select an artifact file, not the deploy directory.")
}

#[cfg(not(unix))]
fn open_image_file(root: &Path, path: &Path) -> Result<fs::File> {
    if fs::symlink_metadata(path)?.file_type().is_symlink()
        || !path.canonicalize()?.starts_with(root.canonicalize()?)
    {
        anyhow::bail!("Artifact viewing does not follow symlinks or escape the deploy directory.");
    }
    Ok(fs::File::open(path)?)
}

fn inspect_image_artifact(
    root: &Path,
    path: &Path,
    dtc: Option<PathBuf>,
) -> Result<ImageArtifactView> {
    if !yoctui_utils::is_absolute_normal_path(path)
        || !yoctui_utils::is_absolute_normal_path(root)
        || !path.starts_with(root)
        || path == root
    {
        anyhow::bail!("Artifact path is outside the authoritative deploy directory.");
    }
    let root_metadata = fs::symlink_metadata(root)?;
    if !root_metadata.is_dir() || root_metadata.file_type().is_symlink() {
        anyhow::bail!("Deploy directory must be a real directory, not a symlink.");
    }
    // Reject symlink ancestors before opening, including nested associated files.
    let mut ancestor = root.to_path_buf();
    for part in path.strip_prefix(root)?.components() {
        ancestor.push(part);
        if fs::symlink_metadata(&ancestor)?.file_type().is_symlink() {
            anyhow::bail!("Artifact viewing does not follow symlinks.");
        }
    }
    let canonical_root = root.canonicalize()?;
    if !path.canonicalize()?.starts_with(&canonical_root) {
        anyhow::bail!("Artifact escapes the authoritative deploy directory.");
    }
    let file = open_image_file(root, path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        anyhow::bail!("Only regular artifact files can be viewed.");
    }
    let kind = match path.extension().and_then(|value| value.to_str()) {
        Some("dtb") => Some(PlatformFileKind::Dtb),
        Some("dtbo") => Some(PlatformFileKind::Dtbo),
        _ => None,
    };
    if let Some(kind) = kind {
        let program = dtc.context("dtc is unavailable. Install device-tree-compiler or add the native dtc tool to the initialized PATH.")?;
        return Ok(ImageArtifactView::DeviceTree {
            kind,
            program,
            size_bytes: metadata.len(),
        });
    }
    if metadata.len() > TEXTAREA_MAX_BYTES as u64 {
        anyhow::bail!(
            "Artifact exceeds the {TEXTAREA_MAX_BYTES}-byte text viewer limit. Use v for RootFS files or an explicit external tool."
        );
    }
    let mut bytes = Vec::new();
    file.take(TEXTAREA_MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > TEXTAREA_MAX_BYTES {
        anyhow::bail!("Artifact grew beyond the text viewer limit.");
    }
    let text = String::from_utf8(bytes).context("Binary artifact cannot be viewed as text. Use v to browse available IMAGE_ROOTFS files; disk images are not mounted.")?;
    if text
        .chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    {
        anyhow::bail!(
            "Binary/control-byte artifact cannot be viewed as text. Use v for available IMAGE_ROOTFS files."
        );
    }
    Ok(ImageArtifactView::Text(text))
}

pub(crate) async fn open_image_artifact(app: &mut App, root: PathBuf, path: PathBuf) {
    let checked_root = root.clone();
    let checked_path = path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let dtc = terminal_launcher::executable_on_initialized_path("dtc")
            .and_then(|path| path.canonicalize().ok());
        inspect_image_artifact(&checked_root, &checked_path, dtc)
    })
    .await
    .map_err(|error| format!("Artifact viewer failed: {error}"))
    .and_then(|result| result.map_err(|error| format!("Could not view artifact: {error:#}")));
    let _ = update(app, Action::ImageArtifactViewed { root, path, result });
}

#[cfg(test)]
#[path = "tests/image_artifact_view.rs"]
mod tests;
