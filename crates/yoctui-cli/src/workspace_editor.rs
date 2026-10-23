//! Workspace editor.
use super::*;

#[cfg(test)]
pub(crate) fn recipe_editor_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn visit(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        if files.len() > yoctui_model::MAX_RECIPE_EDITOR_FILES {
            return Ok(());
        }
        for entry in fs::read_dir(directory)? {
            if files.len() > yoctui_model::MAX_RECIPE_EDITOR_FILES {
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
    if app.workbench.recovering {
        app.workbench.deferred_recipe = Some((recipe, root));
        app.screen = Screen::Workbench;
        app.focus = yoctui_model::FocusTarget::Workspace;
        return;
    }
    // Validate just the root. The Workbench expands directories lazily instead
    // of recursively walking an entire kernel or Poky checkout before opening.
    match tokio::task::spawn_blocking(move || root.canonicalize()).await {
        Ok(Ok(root)) => {
            let _ = update(
                app,
                Action::Workbench(yoctui_model::WorkbenchAction::OpenRecipe { recipe, root }),
            );
        }
        result => app.notification = Some(format!("Could not open recipe workspace: {result:?}")),
    }
}

pub(crate) async fn open_single_workspace_file(app: &mut App, label: String, path: PathBuf) {
    let Some(root) = path.parent().map(Path::to_path_buf) else {
        app.notification = Some("The generated DTS has no parent directory.".into());
        return;
    };
    let Some(file) = path.file_name().map(PathBuf::from) else {
        app.notification = Some("The generated DTS has no filename.".into());
        return;
    };
    if let Some(Effect::LoadRecipeEditorFile(path)) = compatibility_workspace_action(
        app,
        Action::OpenRecipeEditor {
            recipe: label,
            root,
            files: vec![file],
        },
    ) {
        if let Some(Dialog::RecipeEditor(editor)) = app.active_dialog_mut() {
            editor.context = yoctui_model::SourceEditorContext::DeviceTree;
            editor.focus = yoctui_model::RecipeEditorFocus::Document;
        }
        load_recipe_editor_file(app, path).await;
    }
}

#[cfg(test)]
#[path = "tests/workspace_editor.rs"]
mod tests;
