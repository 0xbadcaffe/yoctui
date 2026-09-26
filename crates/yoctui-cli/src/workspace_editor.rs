//! Workspace editor.
use super::*;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_editor_discovers_more_than_the_old_visible_file_limit() {
        let root = std::env::temp_dir().join(format!(
            "yoctui-workspace-editor-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        for index in 0..600 {
            fs::write(
                root.join("src").join(format!("file-{index:03}.c")),
                "int value;\n",
            )
            .unwrap();
        }
        let files = recipe_editor_files(&root).unwrap();
        assert_eq!(files.len(), 600);
        assert!(files.contains(&PathBuf::from("src/file-599.c")));
        fs::remove_dir_all(root).unwrap();
    }
}
