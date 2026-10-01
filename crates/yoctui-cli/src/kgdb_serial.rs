//! Non-opening preparation and confirmed native serial-GDB handoff.
use anyhow::{Context, Result, bail};
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    path::Path,
};
use yoctui_model::{KgdbConfigReport, KgdbSerialSpec, MAX_KGDB_CONFIG_BYTES};

fn regular_input(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    }
    if !fs::symlink_metadata(path)?.is_file() {
        bail!(
            "{} must be a regular file, not a symlink/device/directory",
            path.display()
        );
    }
    let file = options
        .open(path)
        .with_context(|| format!("Cannot read {}", path.display()))?;
    if !file.metadata()?.is_file() {
        bail!("KGDB input changed to a non-regular file");
    }
    Ok(file)
}

pub(crate) fn validate_files(spec: &KgdbSerialSpec) -> Result<KgdbConfigReport> {
    spec.validate().map_err(anyhow::Error::msg)?;
    if !cfg!(target_os = "linux") {
        bail!("This KGDB serial client currently requires a Linux host");
    }
    if !spec.cwd.is_dir() {
        bail!("KGDB host working directory is unavailable");
    }
    if !crate::terminal_launcher::executable(
        &fs::metadata(&spec.gdb).context("GDB is unavailable")?,
    ) {
        bail!("Selected GDB is not executable");
    }
    let mut text = String::new();
    regular_input(&spec.config)?
        .take(MAX_KGDB_CONFIG_BYTES as u64 + 1)
        .read_to_string(&mut text)?;
    let report = KgdbConfigReport::inspect(&text).map_err(anyhow::Error::msg)?;
    crate::qemu_debug::validate_symbol_file(&mut regular_input(&spec.symbols)?)?;
    validate_device(&spec.device)?;
    Ok(report)
}

fn validate_device(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("Host serial device unavailable: {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if metadata.file_type().is_char_device() {
            return Ok(());
        }
    }
    let _ = metadata;
    bail!("Host serial path must be an existing character device, not a symlink/regular file");
}

pub(crate) fn decode(encoded: &str) -> Result<KgdbSerialSpec> {
    if encoded.len() > 32 * 1024 {
        bail!("KGDB specification exceeds 32 KiB");
    }
    serde_json::from_str(encoded).context("Invalid closed KGDB serial specification")
}

pub(crate) fn run(encoded: &str) -> Result<()> {
    let spec = decode(encoded)?;
    validate_files(&spec)?;
    eprintln!(
        "KGDB serial target {} via {}. No automatic halt/reset/resume; after continue, kgdboc re-entry may need manual SysRq-G. Use detach deliberately before leaving.",
        spec.target_uart,
        spec.device.display()
    );
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = std::process::Command::new(&spec.gdb)
            .args(spec.gdb_arguments())
            .current_dir(&spec.cwd)
            .exec();
        Err(error).context("Cannot start the reviewed GDB serial client")
    }
    #[cfg(not(unix))]
    bail!("KGDB serial currently requires Linux");
}

#[cfg(test)]
#[path = "tests/kgdb_serial.rs"]
mod tests;
