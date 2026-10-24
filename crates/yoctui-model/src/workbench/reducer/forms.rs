use super::*;

pub(super) fn confirm_form(app: &mut App) -> Option<Effect> {
    let form = app.workbench.form.clone()?;
    let value = form.text.trim();
    if value.is_empty() && form.kind == WorkbenchFormKind::Find {
        if let Some(s) = app.workbench.current_mut()
            && let Some(t) = s.tabs.get_mut(s.tab)
        {
            let _ = t.document.search("", false);
        }
        app.workbench.form = None;
        return None;
    }
    if value.is_empty() {
        app.notification = Some("Enter a value, or press Esc to cancel.".into());
        return None;
    }
    match form.kind {
        WorkbenchFormKind::Create => {
            if app.workbench.sessions.len() >= WORKBENCH_MAX_SESSIONS || value.len() > 128 {
                app.notification = Some(
                    "At most 16 workspaces are supported; names must be at most 128 bytes.".into(),
                );
                return None;
            }
            app.workbench.generation += 1;
            let id = app.workbench.generation;
            app.workbench.sessions.push(WorkbenchSession {
                id,
                name: value.into(),
                build_dir: app.workspace.build_dir.clone(),
                recipe: None,
                folders: Vec::new(),
                folder: 0,
                tabs: Vec::new(),
                tab: 0,
                focus: WorkbenchFocus::Files,
                output: WorkbenchOutput::Build,
                panel_zoom: false,
                terminal: None,
                terminal_daemon: None,
                show_whitespace: false,
                debug: WorkbenchDebugConfig::default(),
            });
            app.workbench.selection = app.workbench.sessions.len() - 1;
            app.workbench.active = Some(id);
            app.workbench.changed();
            return reduce(app, WorkbenchAction::Form(WorkbenchFormKind::AddFolder));
        }
        WorkbenchFormKind::Rename => {
            if value.len() > 128 {
                return None;
            }
            let index = app
                .workbench
                .current()
                .and_then(|s| {
                    app.workbench
                        .sessions
                        .iter()
                        .position(|other| other.id == s.id)
                })
                .unwrap_or(app.workbench.selection);
            if let Some(s) = app.workbench.sessions.get_mut(index) {
                s.name = value.into();
            }
        }
        WorkbenchFormKind::AddFolder => {
            let s = app.workbench.current()?;
            if s.folders.len() >= WORKBENCH_MAX_FOLDERS {
                app.notification = Some("A workspace supports up to eight folders.".into());
                return None;
            }
            let path = PathBuf::from(value);
            if !path.is_absolute() {
                app.notification = Some("Enter an absolute folder path.".into());
                return None;
            }
            let id = s.id;
            app.workbench.generation += 1;
            app.workbench
                .pending_folders
                .insert(app.workbench.generation, id);
            app.workbench.form = None;
            app.workbench.folder_browser = None;
            return Some(Effect::Workbench(WorkbenchRequest::Folder {
                session: id,
                generation: app.workbench.generation,
                path,
            }));
        }
        WorkbenchFormKind::Recipe => {
            if !app.workspace.recipes.iter().any(|r| r.name == value) {
                app.notification = Some("Choose a recipe reported by the attached build. Refresh Recipes if it is missing.".into());
                return None;
            }
            let build_dir = app.workspace.build_dir.clone();
            if let Some(s) = app.workbench.current_mut() {
                s.recipe = Some(value.into());
                s.build_dir = build_dir;
            }
        }
        WorkbenchFormKind::NewFile => {
            let s = app.workbench.current()?;
            let f = s.folders.get(s.folder)?;
            let relative = PathBuf::from(value);
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                app.notification = Some(
                    "Use a relative file path inside the selected folder, without '..'.".into(),
                );
                return None;
            }
            let request = WorkbenchRequest::NewFile {
                session: s.id,
                root: f.browser.root.clone(),
                path: f.browser.root.join(relative),
            };
            app.workbench.form = None;
            return Some(Effect::Workbench(request));
        }
        WorkbenchFormKind::Find => {
            if let Some(s) = app.workbench.current_mut() {
                s.focus = WorkbenchFocus::Editor;
                if let Some(t) = s.tabs.get_mut(s.tab) {
                    let _ = t.document.search(value.to_owned(), false);
                    t.viewport.follow_cursor = true;
                    t.reveal_cursor();
                }
            }
        }
    }
    app.workbench.form = None;
    app.workbench.changed();
    None
}
