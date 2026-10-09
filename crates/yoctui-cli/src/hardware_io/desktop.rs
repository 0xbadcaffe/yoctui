//! Explicit, shell-free handoff of a validated PDF to the desktop reader.
use super::*;

pub(super) async fn open(
    document: yoctui_model::HardwareDocument,
    root: Option<PathBuf>,
) -> Result<()> {
    let path =
        tokio::task::spawn_blocking(move || validated_pdf(&document, root.as_deref())).await??;
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        bail!(
            "A graphical desktop session is required; use the embedded page/text viewer over SSH."
        );
    }
    // gio returns after handing the local file to the registered application.
    // Never execute the document or interpret its path as shell text/a URL.
    if !program_exists("gio") {
        bail!("Install GLib gio and a desktop PDF reader (for example Evince or Okular).");
    }
    run_command("gio", [std::ffi::OsStr::new("open"), path.as_os_str()]).await?;
    Ok(())
}

pub(super) fn validated_pdf(
    document: &yoctui_model::HardwareDocument,
    root: Option<&Path>,
) -> Result<PathBuf> {
    if document.kind != HardwareDocumentKind::Pdf {
        bail!("Only PDF documents can be opened in the desktop PDF reader.");
    }
    validate_source(&document.path, HardwareDocumentKind::Pdf)?;
    let mut header = Vec::new();
    projects::regular_file(&document.path)?
        .take(1024)
        .read_to_end(&mut header)?;
    if !header.windows(5).any(|bytes| bytes == b"%PDF-") {
        bail!("This file has no PDF header; refusing to open it in a desktop application.");
    }
    if let Some(root) = root {
        projects::validate_preview(root, &document.path)?;
    }
    fs::canonicalize(&document.path).context("Could not resolve the PDF path")
}

#[cfg(test)]
#[path = "../tests/hardware_desktop.rs"]
mod tests;
