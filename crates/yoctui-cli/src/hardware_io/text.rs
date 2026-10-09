use super::*;

pub(super) fn file_kind(path: &Path, project: bool) -> Option<HardwareDocumentKind> {
    let kind = if project {
        HardwareDocumentKind::project_kind(path)
    } else {
        HardwareDocumentKind::library_kind(path)
    }?;
    if kind != HardwareDocumentKind::Text {
        return Some(kind);
    }
    let readable = (|| -> Result<bool> {
        let mut prefix = Vec::new();
        projects::regular_file(path)?
            .take(4096)
            .read_to_end(&mut prefix)?;
        let text = match std::str::from_utf8(&prefix) {
            Ok(text) => text,
            Err(error) if prefix.len() == 4096 && error.error_len().is_none() => {
                std::str::from_utf8(&prefix[..error.valid_up_to()])?
            }
            Err(_) => return Ok(false),
        };
        Ok(yoctui_model::hardware_source_is_text(text))
    })()
    .unwrap_or(false);
    readable.then_some(kind)
}

pub(crate) fn source_editor_content(path: &Path) -> Result<String> {
    let file = projects::regular_file(path)?;
    let limit = yoctui_model::TEXTAREA_MAX_BYTES;
    anyhow::ensure!(
        file.metadata()?.len() <= limit as u64,
        "Hardware source exceeds the 1 MiB editor limit."
    );
    let mut content = String::new();
    file.take(limit as u64 + 1)
        .read_to_string(&mut content)
        .with_context(|| format!("Hardware source is not UTF-8 text: {}", path.display()))?;
    anyhow::ensure!(
        content.len() <= limit,
        "Hardware source grew beyond the 1 MiB editor limit."
    );
    anyhow::ensure!(
        yoctui_model::hardware_source_is_text(&content),
        "Hardware binary/control data cannot be edited as text."
    );
    Ok(content)
}

pub(crate) fn validate_editor_path(
    context: yoctui_model::SourceEditorContext,
    root: &Path,
    path: &Path,
) -> Result<()> {
    projects::validate_editor_directory(root)?;
    if context == yoctui_model::SourceEditorContext::HardwareProject {
        projects::validate_preview(root, path)?;
    } else if matches!(
        context,
        yoctui_model::SourceEditorContext::DeviceTree | yoctui_model::SourceEditorContext::Rootfs
    ) {
        anyhow::ensure!(
            path.is_absolute() && path.starts_with(root),
            "source file escapes its editor root"
        );
        projects::validate_editor_directory(path.parent().context("source file has no parent")?)?;
    }
    source_editor_content(path)?;
    Ok(())
}

pub(super) fn bounded_lines(text: &str) -> Vec<String> {
    let mut bytes = 0usize;
    text.lines()
        .take_while(|line| {
            bytes = bytes.saturating_add(line.len() + 1);
            bytes <= MAX_HARDWARE_TEXT_BYTES
        })
        .map(|line| {
            line.chars()
                .filter(|character| *character == '\t' || !character.is_control())
                .collect()
        })
        .collect()
}

pub(super) fn readable_pdf_lines(text: &str) -> Vec<String> {
    let cleaned = text
        .chars()
        .filter(|character| {
            matches!(character, '\n' | '\t')
                || (!character.is_control()
                    && !is_private_use(*character)
                    && *character != '\u{fffd}')
        })
        .collect::<String>();
    if !cleaned.chars().any(char::is_alphanumeric) {
        return Vec::new();
    }
    bounded_lines(&cleaned)
}

fn is_private_use(character: char) -> bool {
    matches!(character as u32, 0xe000..=0xf8ff | 0xf0000..=0xffffd | 0x100000..=0x10fffd)
}
