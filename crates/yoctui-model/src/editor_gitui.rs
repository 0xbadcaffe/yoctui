//! Context-bound editor GitUI preparation and lossless embedded handoff.
use crate::*;
use std::path::PathBuf;

pub(crate) fn begin(app: &mut App) -> Option<Effect> {
    let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
        return None;
    };
    let root = editor.root.clone();
    let recipe = editor.recipe.clone();
    if app
        .layer_browser
        .as_ref()
        .is_some_and(|browser| browser.is_rootfs() && browser.root == root)
    {
        app.notification = Some("Generated RootFS is image content, not a source Git worktree; open GitUI from its source workspace.".into());
        return None;
    }
    let Some(program) = app.gitui_program.clone() else {
        app.notification = Some("Install GitUI before opening the editor repository.".into());
        return None;
    };
    if !root.is_absolute() {
        app.notification = Some("The editor repository root must be absolute.".into());
        return None;
    }
    let status = app.devtool_statuses.values().find(|status| {
        status.identity.name == recipe
            && matches!(&status.workspace,
            DevtoolWorkspace::Present { source_path, .. } if source_path == &root)
    });
    let cwd = status
        .and_then(|status| match &status.git {
            DevtoolGitState::Available {
                repository_root: Some(repository_root),
                ..
            } if repository_root.is_absolute() => Some(repository_root.clone()),
            _ => None,
        })
        .unwrap_or_else(|| root.clone());
    let name = if status.is_some() {
        format!("GitUI · devtool {recipe}")
    } else {
        format!("GitUI · editor {recipe}")
    };
    app.editor_gitui_generation = app.editor_gitui_generation.wrapping_add(1);
    app.editor_gitui_pending = Some(TerminalLaunchRequest {
        name,
        kind: TerminalCreationKind::GitUi,
        program,
        cwd: cwd.clone(),
        arguments: Vec::new(),
        completion: None,
    });
    app.notification = Some("Inspecting editor repository for GitUI…".into());
    Some(Effect::InspectRecipeEditorGitUi {
        generation: app.editor_gitui_generation,
        root,
        cwd,
    })
}

pub(crate) fn finish(
    app: &mut App,
    generation: u64,
    root: PathBuf,
    result: SourceGitStatus,
) -> Option<Effect> {
    if generation != app.editor_gitui_generation {
        return None;
    }
    let request = app.editor_gitui_pending.take()?;
    if !matches!(app.active_dialog(), Some(Dialog::RecipeEditor(editor)) if editor.root == root)
        || app.menu.is_open()
        || app.command_palette_open
        || app.onboarding.open
    {
        return None;
    }
    match result {
        SourceGitStatus::Ready(_) => {
            if app.gitui_program.as_ref() != Some(&request.program) {
                app.notification =
                    Some("GitUI executable changed; retry the editor launch.".into());
                return None;
            }
            app.notification = None;
            app.dialogs
                .push_front(Dialog::TerminalLaunch(TerminalLaunchDialog {
                    request,
                    destination: TerminalLaunchDestination::Embedded,
                    output_must_not_exist: None,
                }));
            app.focus = FocusTarget::Dialog;
        }
        SourceGitStatus::Unavailable(reason) => {
            app.notification = Some(format!("Cannot open editor GitUI: {reason}"));
        }
        _ => {
            app.notification =
                Some("Editor Git inspection did not return repository authority; retry.".into())
        }
    }
    None
}

pub(crate) fn suspend(app: &mut App) {
    if matches!(app.active_dialog(), Some(Dialog::RecipeEditor(_)))
        && let Some(Dialog::RecipeEditor(editor)) = app.dialogs.pop_front()
    {
        app.suspended_recipe_editor = Some((app.screen, Box::new(editor)));
    }
}

pub(crate) fn restore(app: &mut App) -> Option<Effect> {
    if app.active_dialog().is_some()
        || app.menu.is_open()
        || app.command_palette_open
        || app.onboarding.open
        || app.keymap_preferences_ui.open
    {
        return None;
    }
    if let Some((screen, editor)) = app.suspended_recipe_editor.take() {
        app.screen = screen;
        app.dialogs.push_front(Dialog::RecipeEditor(*editor));
        app.focus = FocusTarget::Dialog;
        app.focus_return = None;
        app.notification =
            Some("Returned to the retained editor; unsaved content was not changed.".into());
    } else {
        app.notification = Some("No editor is retained from a GitUI session.".into());
    }
    None
}

pub(crate) fn retain_dirty_editor(app: &mut App) -> bool {
    if app
        .suspended_recipe_editor
        .as_ref()
        .is_some_and(|(_, editor)| editor.is_dirty())
    {
        restore(app);
        app.notification =
            Some("Save the retained editor with Ctrl+S before opening another workspace.".into());
        true
    } else {
        false
    }
}

#[cfg(test)]
#[path = "tests/editor_gitui.rs"]
mod tests;
