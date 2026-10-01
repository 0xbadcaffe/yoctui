use super::*;

pub(super) async fn load_export(
    path: &Path,
    page: usize,
) -> Result<(usize, HardwarePreview, Vec<String>)> {
    // Proprietary schematic binaries are not generic images. Use only an
    // explicit sibling PDF export; never execute an associated desktop app.
    for extension in ["pdf", "PDF"] {
        let pdf = path.with_extension(extension);
        if pdf.exists() {
            validate_source(&pdf, HardwareDocumentKind::Pdf)?;
            return load_pdf(&pdf, page).await;
        }
    }
    let (text, source_error) = match bounded_source_text(path) {
        Ok(text) => (text, String::new()),
        Err(error) => (Vec::new(), format!(" Source preview unavailable: {error}")),
    };
    let limitation = format!(
        "Source only: native Altium/Xpedition rendering is unavailable. Export a same-stem PDF ({}) into this folder for graphical viewing.{source_error}",
        path.with_extension("pdf")
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    );
    Ok((
        1,
        HardwarePreview::Text {
            lines: text.clone(),
            limitation: Some(limitation),
        },
        text,
    ))
}
