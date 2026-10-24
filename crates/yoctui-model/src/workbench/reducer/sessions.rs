use super::*;

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
                    show_whitespace: false,
                    debug: WorkbenchDebugConfig::default(),
                });
                app.workbench.sessions.len() - 1
            };
            app.workbench.selection = index;
            app.workbench.active = Some(app.workbench.sessions[index].id);
            app.screen = Screen::Workbench;
            app.focus = FocusTarget::Workspace;
            app.workbench.changed();
        }
        W::Sessions => {
            app.workbench.active = None;
            app.workbench.form = None;
            app.workbench.folder_browser = None;
            app.workbench.menu = None;
            app.workbench.tab_picker = None;
            app.workbench.debug_dialog = None;
            app.workbench.pending_compile = None;
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
        _ => {}
    }
    None
}
