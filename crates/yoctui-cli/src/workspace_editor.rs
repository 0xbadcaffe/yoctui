//! Workspace editor.
use super::*;

pub(crate) fn recipe_editor_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn visit(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in fs::read_dir(directory)? {
            if files.len() >= 512 {
                break;
            }
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                if entry.file_name() != ".git" {
                    visit(root, &path, files)?;
                }
            } else if file_type.is_file()
                && entry.metadata()?.len() <= 1_048_576
                && let Ok(relative) = path.strip_prefix(root)
            {
                files.push(relative.to_path_buf());
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

pub(crate) async fn open_workspace_editor(app: &mut App, recipe: String, root: PathBuf) {
    let root_for_scan = root.clone();
    let files = tokio::task::spawn_blocking(move || recipe_editor_files(&root_for_scan)).await;
    match files {
        Ok(Ok(files)) => {
            if let Some(Effect::LoadRecipeEditorFile(path)) = compatibility_workspace_action(
                app,
                Action::OpenRecipeEditor {
                    recipe,
                    root,
                    files,
                },
            ) {
                load_recipe_editor_file(app, path).await;
            }
        }
        Ok(Err(error)) => {
            app.notification = Some(format!("Could not list workspace files: {error}"))
        }
        Err(error) => app.notification = Some(format!("Workspace file scan failed: {error}")),
    }
}
