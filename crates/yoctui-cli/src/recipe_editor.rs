//! Recipe editor.
use super::*;
mod gitui;
pub(crate) use gitui::EditorGitUiIo;

pub(crate) fn read_source_editor_file(
    editor: &yoctui_model::RecipeEditor,
    path: &Path,
) -> Result<String> {
    if editor.context != yoctui_model::SourceEditorContext::Recipe {
        hardware_io::text::validate_editor_path(editor.context, &editor.root, path)?;
        return hardware_io::text::source_editor_content(path);
    }
    Ok(fs::read_to_string(path)?)
}

pub(crate) async fn load_recipe_editor_file(app: &mut App, path: PathBuf) {
    let context = app.active_dialog().and_then(|dialog| match dialog {
        Dialog::RecipeEditor(editor) => Some((editor.context, editor.root.clone())),
        _ => None,
    });
    let result = tokio::task::spawn_blocking(move || -> Result<String> {
        if let Some((context, root)) = context
            && context != yoctui_model::SourceEditorContext::Recipe
        {
            hardware_io::text::validate_editor_path(context, &root, &path)?;
            return hardware_io::text::source_editor_content(&path);
        }
        Ok(fs::read_to_string(path)?)
    })
    .await;
    match result {
        Ok(Ok(content)) => {
            let _ = update(app, Action::LoadRecipeEditorContent(content));
        }
        Ok(Err(error)) => app.notification = Some(format!("Could not read recipe file: {error}")),
        Err(error) => app.notification = Some(format!("Recipe file load failed: {error}")),
    }
}

pub(crate) fn write_recipe_editor_file_atomically(
    root: &Path,
    path: &Path,
    content: &str,
    expected: TextAreaRevision,
) -> Result<()> {
    if content.len() > TEXTAREA_MAX_BYTES {
        anyhow::bail!("source file exceeds the {TEXTAREA_MAX_BYTES}-byte editor limit");
    }
    let canonical_root = root
        .canonicalize()
        .with_context(|| format!("could not resolve workspace root {}", root.display()))?;
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("could not inspect {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        anyhow::bail!("refusing to replace a symlink or non-regular workspace file");
    }
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("could not resolve {}", path.display()))?;
    if !canonical_path.starts_with(&canonical_root) {
        anyhow::bail!("refusing to save outside the Devtool workspace root");
    }
    let current = fs::read_to_string(&canonical_path)
        .with_context(|| format!("could not read {} before saving", canonical_path.display()))?;
    if TextAreaRevision::of(&current) != expected {
        anyhow::bail!(
            "the file changed on disk; reload it before saving to avoid overwriting external edits"
        );
    }
    let parent = canonical_path
        .parent()
        .context("workspace source file has no parent directory")?;
    let file_name = canonical_path
        .file_name()
        .and_then(|value| value.to_str())
        .context("workspace source filename is not valid UTF-8")?;
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(
        ".{file_name}.yoctui-{}-{nonce}.tmp",
        std::process::id()
    ));
    let save = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .with_context(|| format!("could not create temporary file in {}", parent.display()))?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        fs::set_permissions(&temporary, metadata.permissions())?;
        fs::rename(&temporary, &canonical_path)?;
        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();
    if save.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    save
}

pub(crate) async fn save_recipe_editor_file(
    app: &mut App,
    root: PathBuf,
    path: PathBuf,
    content: String,
    expected: TextAreaRevision,
) {
    let context = app
        .active_dialog()
        .and_then(|dialog| match dialog {
            Dialog::RecipeEditor(editor) => Some(editor.context),
            _ => None,
        })
        .unwrap_or_default();
    let result = tokio::task::spawn_blocking(move || {
        if context != yoctui_model::SourceEditorContext::Recipe {
            hardware_io::text::validate_editor_path(context, &root, &path)?;
            anyhow::ensure!(
                yoctui_model::hardware_source_is_text(&content),
                "Hardware binary/control data cannot be saved as text."
            );
        }
        write_recipe_editor_file_atomically(&root, &path, &content, expected)
    })
    .await;
    match result {
        Ok(Ok(())) => {
            let _ = update(app, Action::RecipeEditorSaved);
        }
        Ok(Err(error)) => app.notification = Some(format!("Could not save recipe file: {error}")),
        Err(error) => app.notification = Some(format!("Recipe file save failed: {error}")),
    }
}
