//! Snapshot-bound standalone fragment export; no config integration or processes.
use anyhow::{Context, Result, bail};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use yoctui_model::{
    KernelInstrumentationDraft, KernelInstrumentationPreview, KernelInstrumentationReport,
    MAX_KERNEL_INSTRUMENTATION_CONFIG_BYTES,
};

pub(crate) fn inspect(draft: &KernelInstrumentationDraft) -> Result<KernelInstrumentationPreview> {
    draft.validate().map_err(anyhow::Error::msg)?;
    let config = Path::new(&draft.config);
    if !fs::symlink_metadata(config)?.is_file() {
        bail!("Kernel config must be a regular file, not a symlink/device/directory.");
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    }
    let file = options
        .open(config)
        .context("Cannot read exact kernel config")?;
    if !file.metadata()?.is_file() {
        bail!("Kernel config changed to a nonregular file.");
    }
    let mut text = String::new();
    file.take(MAX_KERNEL_INSTRUMENTATION_CONFIG_BYTES as u64 + 1)
        .read_to_string(&mut text)?;
    let report =
        KernelInstrumentationReport::inspect(draft.preset, &text).map_err(anyhow::Error::msg)?;
    let output = Path::new(&draft.output);
    if yoctui_utils::path_entry_exists(output)? {
        bail!(
            "Refusing to overwrite {}; choose a new .cfg destination.",
            output.display()
        );
    }
    let parent = output
        .parent()
        .context("Export destination has no parent")?;
    if !fs::symlink_metadata(parent)?.is_dir() {
        bail!("Export parent must be a real directory, not a symlink.");
    }
    let destination_parent = parent.canonicalize()?;
    let metadata = fs::metadata(&destination_parent)?;
    #[cfg(unix)]
    let parent_identity = {
        use std::os::unix::fs::MetadataExt;
        Some((metadata.dev(), metadata.ino()))
    };
    #[cfg(not(unix))]
    let parent_identity = {
        let _ = metadata;
        None
    };
    Ok(KernelInstrumentationPreview {
        draft: draft.clone(),
        report,
        destination_parent,
        parent_identity,
    })
}

fn create_output(expected: &KernelInstrumentationPreview) -> Result<File> {
    #[cfg(unix)]
    {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::{
                ffi::OsStrExt,
                fs::{MetadataExt, OpenOptionsExt},
            },
        };
        let parent = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC)
            .open(&expected.destination_parent)?;
        let metadata = parent.metadata()?;
        if Some((metadata.dev(), metadata.ino())) != expected.parent_identity {
            bail!("Export parent changed since review; inspect again.");
        }
        let name = Path::new(&expected.draft.output)
            .file_name()
            .context("Missing export filename")?;
        let name = std::ffi::CString::new(name.as_bytes())?;
        // Descriptor-relative exclusive creation anchors the reviewed parent and
        // atomically refuses existing entries, including dangling symlinks.
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // Successful openat returns a fresh descriptor exclusively owned here.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
    #[cfg(not(unix))]
    {
        Ok(OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&expected.draft.output)?)
    }
}

pub(crate) fn export(expected: &KernelInstrumentationPreview) -> Result<PathBuf> {
    let current = inspect(&expected.draft)?;
    if &current != expected {
        bail!("Kernel config or export parent changed since review; inspect again.");
    }
    let mut file = create_output(expected).context("Could not create the reviewed NEW fragment")?;
    file.write_all(expected.draft.preset.fragment().as_bytes())
        .and_then(|_| file.sync_all())
        .with_context(|| {
            format!(
                "Export failed; {} may contain a partial fragment. Input config was not changed.",
                expected.draft.output
            )
        })?;
    Ok(expected.draft.output.clone().into())
}

#[cfg(test)]
#[path = "tests/kernel_instrumentation.rs"]
mod tests;
