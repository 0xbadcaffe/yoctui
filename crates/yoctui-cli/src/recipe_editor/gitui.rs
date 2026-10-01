//! Bounded read-only inspection; the input loop never waits for Git.
use super::*;
use yoctui_model::SourceGitStatus;

#[derive(Default)]
pub(crate) struct EditorGitUiIo {
    pending: Option<(u64, PathBuf, tokio::task::JoinHandle<SourceGitStatus>)>,
}

impl Drop for EditorGitUiIo {
    fn drop(&mut self) {
        if let Some((_, _, task)) = self.pending.take() {
            task.abort();
        }
    }
}

impl EditorGitUiIo {
    pub(crate) fn submit(&mut self, effect: Effect) {
        let Effect::InspectRecipeEditorGitUi {
            generation,
            root,
            cwd,
        } = effect
        else {
            return;
        };
        if let Some((_, _, task)) = self.pending.take() {
            task.abort();
        }
        let checked_root = root.clone();
        self.pending = Some((
            generation,
            root,
            tokio::spawn(async move {
                tokio::time::timeout(Duration::from_secs(8), async {
                    match validate_editor_gitui_paths(&checked_root, &cwd).await {
                        Ok(()) => yoctui_bitbake::inspect_source_git(&cwd).await,
                        Err(error) => SourceGitStatus::Unavailable(error.to_string()),
                    }
                })
                .await
                .unwrap_or_else(|_| {
                    SourceGitStatus::Unavailable("Editor Git inspection timed out".into())
                })
            }),
        ));
    }

    pub(crate) async fn poll(&mut self, app: &mut App) -> bool {
        let Some((generation, root, task)) = self.pending.as_ref() else {
            return false;
        };
        if *generation != app.editor_gitui_generation
            || !matches!(app.active_dialog(), Some(Dialog::RecipeEditor(editor)) if &editor.root == root)
            || app.menu.is_open()
            || app.command_palette_open
            || app.onboarding.open
        {
            let (generation, _, task) = self.pending.take().expect("checked pending Git probe");
            task.abort();
            if generation == app.editor_gitui_generation {
                app.editor_gitui_pending = None;
                app.notification =
                    Some("Editor GitUI inspection cancelled; retry from the editor.".into());
            }
            return true;
        }
        if !task.is_finished() {
            return false;
        }
        let (generation, root, task) = self.pending.take().expect("finished Git probe");
        let result = task
            .await
            .unwrap_or_else(|error| SourceGitStatus::Unavailable(error.to_string()));
        update(
            app,
            Action::RecipeEditorGitUiInspected {
                generation,
                root,
                result,
            },
        );
        true
    }
}

async fn validate_editor_gitui_paths(root: &Path, cwd: &Path) -> Result<()> {
    for path in [root, cwd] {
        if !path.is_absolute()
            || path.components().count() > 64
            || path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            anyhow::bail!("editor GitUI needs bounded absolute directory paths without traversal");
        }
        for ancestor in path.ancestors() {
            let metadata = tokio::fs::symlink_metadata(ancestor)
                .await
                .with_context(|| {
                    format!("cannot inspect editor directory {}", ancestor.display())
                })?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                anyhow::bail!(
                    "editor GitUI refuses symlink or non-directory path {}",
                    ancestor.display()
                );
            }
        }
    }
    let root = tokio::fs::canonicalize(root).await?;
    let cwd = tokio::fs::canonicalize(cwd).await?;
    if !root.starts_with(cwd) {
        anyhow::bail!("reported Git repository is not an ancestor of the editor root");
    }
    Ok(())
}

#[cfg(test)]
#[path = "gitui_tests.rs"]
mod tests;
