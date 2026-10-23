use super::*;
mod forms;
mod results;
mod tree;
use forms::confirm_form;
use tree::{directory_request, open_entry};

pub(super) fn reduce(app: &mut App, action: WorkbenchAction) -> Option<Effect> {
    use WorkbenchAction as W;
    match action {
        W::OpenRecipe { recipe, root } => {
            if !root.is_absolute() {
                app.notification =
                    Some("Recipe workspace must have an absolute source folder.".into());
                return None;
            }
            let existing = app.workbench.sessions.iter().position(|s| {
                s.build_dir == app.workspace.build_dir
                    && s.recipe.as_ref() == Some(&recipe)
                    && s.folders.iter().any(|f| f.browser.root == root)
            });
            let index = if let Some(index) = existing {
                index
            } else {
                if app.workbench.sessions.len() >= WORKBENCH_MAX_SESSIONS {
                    app.notification =
                        Some("Remove a saved workspace session before opening another.".into());
                    return None;
                }
                app.workbench.generation = app.workbench.generation.saturating_add(1);
                app.workbench.sessions.push(WorkbenchSession {
                    id: app.workbench.generation,
                    name: recipe.clone(),
                    recipe: Some(recipe),
                    build_dir: app.workspace.build_dir.clone(),
                    folders: vec![WorkbenchFolder {
                        browser: LayerBrowser::new("Workbench".into(), root),
                        request: 0,
                        requests: HashMap::new(),
                        loading: false,
                        error: None,
                    }],
                    folder: 0,
                    tabs: vec![],
                    tab: 0,
                    focus: WorkbenchFocus::Files,
                    output: WorkbenchOutput::Build,
                    panel_zoom: false,
                    terminal: None,
                    terminal_daemon: None,
                });
                app.workbench.sessions.len() - 1
            };
            app.workbench.selection = index;
            app.workbench.active = Some(app.workbench.sessions[index].id);
            app.screen = Screen::Workbench;
            app.focus = FocusTarget::Workspace;
            app.workbench.changed();
        }
        W::FolderLoaded { .. }
        | W::DirectoryLoaded { .. }
        | W::FileLoaded { .. }
        | W::Saved { .. }
        | W::NewFileCreated { .. } => return results::reduce(app, action),
        W::Form(kind) => {
            let text = match kind {
                WorkbenchFormKind::Rename => app
                    .workbench
                    .current()
                    .or_else(|| app.workbench.sessions.get(app.workbench.selection))
                    .map(|s| s.name.clone())
                    .unwrap_or_default(),
                WorkbenchFormKind::AddFolder => app
                    .workspace
                    .source_dir
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                WorkbenchFormKind::Recipe => app
                    .workbench
                    .current()
                    .and_then(|s| s.recipe.clone())
                    .unwrap_or_default(),
                _ => String::new(),
            };
            app.workbench.form = Some(WorkbenchForm { kind, text });
            app.workbench.menu = None;
        }
        W::FormText(c) if !c.is_control() => {
            if let Some(f) = &mut app.workbench.form
                && f.text.len() + c.len_utf8() <= 4096
            {
                f.text.push(c);
            }
        }
        W::FormText(_) => {}
        W::FormBackspace => {
            if let Some(f) = &mut app.workbench.form {
                f.text.pop();
            }
        }
        W::FormClear => {
            if let Some(f) = &mut app.workbench.form {
                f.text.clear();
            }
        }
        W::ConfirmForm => return confirm_form(app),
        W::Cancel => {
            if app.workbench.form.take().is_some() || app.workbench.menu.take().is_some() {
                return None;
            }
            if let Some(s) = app.workbench.current_mut() {
                if s.focus != WorkbenchFocus::Files {
                    s.focus = WorkbenchFocus::Files;
                } else {
                    app.workbench.active = None;
                }
            } else {
                app.focus = FocusTarget::Navigator;
            }
        }
        W::Select(delta) => {
            if let Some(s) = app.workbench.current_mut() {
                if let Some(f) = s.folders.get_mut(s.folder) {
                    f.browser.selection =
                        shifted_index(f.browser.selection, delta, f.browser.entries.len());
                }
            } else {
                app.workbench.selection =
                    shifted_index(app.workbench.selection, delta, app.workbench.sessions.len());
            }
        }
        W::SelectAt(index) => {
            app.focus = FocusTarget::Workspace;
            if let Some(s) = app.workbench.current_mut() {
                if let Some(f) = s.folders.get_mut(s.folder) {
                    f.browser.selection = index.min(f.browser.entries.len().saturating_sub(1));
                }
            } else {
                app.workbench.selection = index.min(app.workbench.sessions.len().saturating_sub(1));
            }
        }
        W::Open => {
            if app.workbench.active.is_none() {
                app.workbench.active = app
                    .workbench
                    .sessions
                    .get(app.workbench.selection)
                    .map(|s| s.id);
                app.focus = FocusTarget::Workspace;
                return directory_request(app, None);
            }
            return open_entry(app);
        }
        W::Sessions => {
            app.workbench.active = None;
            app.workbench.form = None;
            app.workbench.menu = None;
        }
        W::RemoveSession => {
            let index = app.workbench.selection;
            if app
                .workbench
                .sessions
                .get(index)
                .is_some_and(|s| s.tabs.iter().any(WorkbenchTab::is_dirty))
            {
                app.notification = Some("Save all workspace changes before removing its session. Files and terminals are never deleted by session removal.".into());
            } else if index < app.workbench.sessions.len() {
                app.workbench.sessions.remove(index);
                app.workbench.selection = index.min(app.workbench.sessions.len().saturating_sub(1));
                app.workbench.changed();
            }
        }
        W::RemoveFolder => {
            if let Some(s) = app.workbench.current_mut()
                && let Some(f) = s.folders.get(s.folder)
            {
                let root = f.browser.root.clone();
                if s.tabs.iter().any(|t| t.root == root) {
                    app.notification = Some("Close this folder's editor tabs before removing it. No files will be deleted.".into());
                    return None;
                }
                s.folders.remove(s.folder);
                s.folder = s.folder.min(s.folders.len().saturating_sub(1));
                app.workbench.changed();
            }
        }
        W::Focus(focus) => {
            app.focus = FocusTarget::Workspace;
            if let Some(s) = app.workbench.current_mut() {
                s.focus = focus;
                if focus == WorkbenchFocus::Editor {
                    s.panel_zoom = false;
                }
            }
        }
        W::CycleFocus => {
            if let Some(s) = app.workbench.current_mut() {
                s.focus = match s.focus {
                    WorkbenchFocus::Files => WorkbenchFocus::Editor,
                    WorkbenchFocus::Editor => WorkbenchFocus::Output,
                    _ => WorkbenchFocus::Files,
                };
            }
        }
        W::Folder(delta) => {
            if let Some(s) = app.workbench.current_mut() {
                s.folder = shifted_index(s.folder, delta, s.folders.len());
                s.focus = WorkbenchFocus::Files;
            }
            return directory_request(app, None);
        }
        W::FolderAt(index) => {
            app.focus = FocusTarget::Workspace;
            if let Some(s) = app.workbench.current_mut() {
                s.folder = index.min(s.folders.len().saturating_sub(1));
                s.focus = WorkbenchFocus::Files;
                app.workbench.changed();
            }
            return directory_request(app, None);
        }
        W::Expand => return open_entry(app),
        W::Collapse => {
            if let Some(f) = app
                .workbench
                .current_mut()
                .and_then(|s| s.folders.get_mut(s.folder))
                && let Some(entry) = f.browser.selected_entry().cloned()
            {
                let path = if entry.is_dir && f.browser.expanded.remove(&entry.path) {
                    entry.path
                } else {
                    entry.path.parent().unwrap_or(&f.browser.root).to_path_buf()
                };
                f.browser.expanded.remove(&path);
                f.browser.rebuild(Some(&path));
            }
        }
        W::Hidden => {
            if let Some(f) = app
                .workbench
                .current_mut()
                .and_then(|s| s.folders.get_mut(s.folder))
            {
                f.browser.show_hidden = !f.browser.show_hidden;
                f.browser.rebuild(None);
            }
        }
        W::Refresh => return directory_request(app, None),
        W::Tab(delta) => {
            if let Some(s) = app.workbench.current_mut() {
                s.tab = shifted_index(s.tab, delta, s.tabs.len());
                s.focus = WorkbenchFocus::Editor;
                s.panel_zoom = false;
                app.workbench.changed();
            }
        }
        W::SelectTab(index) => {
            app.focus = FocusTarget::Workspace;
            if let Some(s) = app.workbench.current_mut() {
                s.tab = index.min(s.tabs.len().saturating_sub(1));
                s.focus = WorkbenchFocus::Editor;
                s.panel_zoom = false;
            }
        }
        W::CloseTab => {
            if let Some(s) = app.workbench.current_mut() {
                if s.tabs.get(s.tab).is_some_and(|t| t.is_dirty() || t.saving) {
                    app.notification = Some("Save changes before closing this tab. Switching tabs or sessions retains unsaved edits.".into());
                } else if s.tab < s.tabs.len() {
                    s.tabs.remove(s.tab);
                    s.tab = s.tab.min(s.tabs.len().saturating_sub(1));
                    app.workbench.changed();
                }
            }
        }
        W::Save => {
            let s = app.workbench.current_mut()?;
            let t = s.tabs.get_mut(s.tab)?;
            if t.saving {
                app.notification = Some(
                    "This file is still saving; newer edits are retained. Retry after completion."
                        .into(),
                );
                return None;
            }
            if !t.is_dirty() {
                app.notification = Some("This file has no unsaved changes.".into());
                return None;
            }
            t.saving = true;
            return Some(Effect::Workbench(WorkbenchRequest::Save {
                session: s.id,
                root: t.root.clone(),
                path: t.path.clone(),
                content: t.document.text.clone(),
                expected: TextAreaRevision::of(&t.original),
            }));
        }
        W::Edit(command) => {
            app.focus = FocusTarget::Workspace;
            if let Some(s) = app.workbench.current_mut() {
                s.focus = WorkbenchFocus::Editor;
            }
            // Include baselines in the global recovery budget; never evict dirty buffers.
            let extra = match &command {
                PopupEditorCommand::Insert(c) => c.len_utf8(),
                PopupEditorCommand::Newline => 1,
                PopupEditorCommand::PasteText { text, .. } => text.len(),
                PopupEditorCommand::Paste => TEXTAREA_MAX_PASTE_BYTES,
                _ => 0,
            };
            let growth = app
                .workbench
                .current()
                .and_then(|s| s.tabs.get(s.tab))
                .map_or(0, |t| {
                    t.document
                        .text
                        .len()
                        .saturating_add(extra)
                        .saturating_sub(t.reserved)
                        .saturating_mul(2)
                });
            if app.workbench.text_bytes().saturating_add(growth) > WORKBENCH_MAX_TEXT {
                app.notification = Some(
                    "Workbench recovery text limit reached; save and close unused tabs.".into(),
                );
                return None;
            }
            if let Some(t) = app
                .workbench
                .current_mut()
                .and_then(|s| s.tabs.get_mut(s.tab))
            {
                t.document.editing = true;
                match crate::editor_types::apply_recipe_editor_command(&mut t.document, command) {
                    Ok(Some(text)) => return Some(Effect::CopyToClipboard(text)),
                    Ok(None) => {}
                    Err(error) => app.notification = Some(error),
                }
                t.document.editing = true;
                t.reserved = t.reserved.max(t.document.text.len());
                t.diff = None;
                app.workbench.changed();
            }
        }
        W::SelectAll => {
            if let Some(t) = app
                .workbench
                .current_mut()
                .and_then(|s| s.tabs.get_mut(s.tab))
            {
                t.document.select_range(0, t.document.text.len());
                t.document.editing = true;
            }
        }
        W::Cut => {
            let session = app.workbench.current_mut()?;
            let t = session.tabs.get_mut(session.tab)?;
            if t.document.selection.is_none() {
                app.notification =
                    Some("Select text with Shift+arrows or Ctrl+a before cutting.".into());
                return None;
            }
            let text = t.document.copy_selection_or_line();
            t.document.delete_forward();
            t.diff = None;
            app.workbench.changed();
            return Some(Effect::CopyToClipboard(text));
        }
        W::Menu => {
            app.workbench.menu = Some(0);
        }
        W::MenuSelect(delta) => {
            app.workbench.menu = Some(shifted_index(
                app.workbench.menu.unwrap_or(0),
                delta,
                WORKBENCH_MENU.len(),
            ));
        }
        W::MenuActivate => {
            let index = app.workbench.menu.take()?;
            let command = match index {
                0 => W::Form(WorkbenchFormKind::AddFolder),
                1 => W::Form(WorkbenchFormKind::NewFile),
                2 => W::Form(WorkbenchFormKind::Recipe),
                3 => W::RecipeAction(Box::new(Action::BeginSelectedRecipeBuild)),
                4 => W::RecipeAction(Box::new(Action::BeginSelectedRecipeDevtoolModify)),
                5 => W::RecipeAction(Box::new(Action::BeginSelectedRecipeDevtoolStatus)),
                6 => W::RecipeAction(Box::new(Action::BeginSelectedRecipeDevshell)),
                7 => W::Terminal,
                8 => W::Changes,
                9 => W::RecipeAction(Box::new(Action::BeginSelectedRecipeDevtoolPatch)),
                10 => W::RecipeAction(Box::new(Action::BeginSelectedRecipeDevtoolFinish)),
                11 => W::RecipeAction(Box::new(Action::BeginSelectedRecipeDevtoolReset)),
                12 => W::Form(WorkbenchFormKind::Rename),
                13 => W::RemoveFolder,
                _ => W::Sessions,
            };
            return reduce(app, command);
        }
        W::Changes => {
            if let Some(s) = app.workbench.current_mut() {
                s.output = WorkbenchOutput::Changes;
                if let Some(t) = s.tabs.get_mut(s.tab) {
                    let mut preview = TextAreaState::new(t.original.clone());
                    preview.select_range(0, preview.text.len());
                    if preview.try_insert(&t.document.text).is_ok() {
                        t.diff = Some(preview.preview_diff().clone());
                    }
                }
                s.focus = WorkbenchFocus::Output;
            }
        }
        W::TogglePanel => {
            if let Some(s) = app.workbench.current_mut() {
                s.panel_zoom = !s.panel_zoom;
            }
        }
        W::Terminal => {
            return crate::workbench::reducer::results::terminal(app);
        }
        W::RecipeAction(action) => {
            let s = app.workbench.current()?;
            if !matches!(
                *action,
                Action::BeginSelectedRecipeBuild
                    | Action::BeginSelectedRecipeDevtoolModify
                    | Action::BeginSelectedRecipeDevtoolStatus
                    | Action::BeginSelectedRecipeDevshell
                    | Action::BeginSelectedRecipeDevtoolPatch
                    | Action::BeginSelectedRecipeDevtoolFinish
                    | Action::BeginSelectedRecipeDevtoolReset
            ) {
                return None;
            }
            if !matches!(*action, Action::BeginSelectedRecipeDevtoolStatus)
                && s.tabs.iter().any(WorkbenchTab::is_dirty)
            {
                app.notification = Some(
                    "Save open changes (Ctrl+s in each edited tab) before running recipe commands."
                        .into(),
                );
                return None;
            }
            if s.build_dir.as_ref() != app.workspace.build_dir.as_ref() {
                app.notification = Some("This workspace belongs to another build environment. Attach that build before running its recipe commands.".into());
                return None;
            }
            let Some(name) = s.recipe.clone() else {
                return reduce(app, W::Form(WorkbenchFormKind::Recipe));
            };
            let Some(index) = app.workspace.recipes.iter().position(|r| r.name == name) else {
                app.notification = Some(format!(
                    "Recipe {name} is not available in the attached build."
                ));
                return None;
            };
            app.recipe_selection = index;
            if matches!(*action, Action::BeginSelectedRecipeBuild)
                && let Some(s) = app.workbench.current_mut()
            {
                s.output = WorkbenchOutput::Build;
                s.panel_zoom = false;
            }
            return update_with_workspace_authority(app, *action);
        }
    }
    None
}
