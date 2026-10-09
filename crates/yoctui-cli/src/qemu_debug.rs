//! Managed QEMU/GDB helper; launched only after the typed terminal review.
use anyhow::{Context, Result, bail};
use yoctui_model::QemuDebugSpec;
mod source_map;
mod validation;
pub(crate) use validation::validate_files;
pub(crate) use validation::validate_symbol_file;
#[cfg(unix)]
mod runtime;

pub(crate) async fn run(encoded: &str) -> Result<()> {
    if encoded.len() > 32 * 1024 {
        bail!("QEMU debug specification exceeds 32 KiB");
    }
    let spec: QemuDebugSpec =
        serde_json::from_str(encoded).context("invalid closed QEMU debug specification")?;
    validate_files(&spec)?;
    #[cfg(unix)]
    {
        runtime::run(&spec).await
    }
    #[cfg(not(unix))]
    {
        bail!("Managed QEMU debugging requires Unix sockets/process groups")
    }
}

#[cfg(test)]
#[path = "tests/qemu_debug.rs"]
mod tests;
