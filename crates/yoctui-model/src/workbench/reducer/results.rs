use super::*;

pub(super) fn reduce(app: &mut App, action: WorkbenchAction) -> Option<Effect> {
    use WorkbenchAction as W;
    match action {
        W::FolderLoaded {
            session,
            generation,
            result,
        } => {
            if app.workbench.pending_folders.remove(&generation) != Some(session) {
                return None;
            }
            let s = app
                .workbench
                .sessions
                .iter_mut()
                .find(|s| s.id == session)?;
            match result {
                Ok(root)
                    if !s.folders.iter().any(|f| f.browser.root == root)
                        && s.folders.len() < WORKBENCH_MAX_FOLDERS =>
                {
                    s.folders.push(WorkbenchFolder {
                        browser: LayerBrowser::new("Workbench".into(), root),
                        request: 0,
                        requests: HashMap::new(),
                        loading: false,
                        error: None,
                    });
                    s.folder = s.folders.len() - 1;
                    app.workbench.changed();
                    if app.workbench.active == Some(session) {
                        return directory_request(app, None);
                    }
                }
                Ok(_) => {
                    app.notification =
                        Some("That folder is already open, or the folder limit was reached.".into())
                }
                Err(error) => {
                    app.notification = Some(format!("Could not add workspace folder: {error}"))
                }
            }
        }
        W::DirectoryLoaded {
            session,
            root,
            directory,
            generation,
            result,
        } => {
            let s = app
                .workbench
                .sessions
                .iter_mut()
                .find(|s| s.id == session)?;
            let f = s.folders.iter_mut().find(|f| {
                f.browser.root == root && f.requests.get(&directory) == Some(&generation)
            })?;
            f.requests.remove(&directory);
            f.loading = !f.requests.is_empty();
            match result {
                Ok(entries)
                    if entries.len() <= MAX_ROOTFS_ENTRIES
                        && directory.starts_with(&root)
                        && entries
                            .iter()
                            .all(|e| e.path.parent() == Some(directory.as_path())) =>
                {
                    let total = f
                        .browser
                        .nodes
                        .iter()
                        .filter(|(p, _)| **p != directory)
                        .map(|(_, v)| v.len())
                        .sum::<usize>();
                    if total.saturating_add(entries.len()) > MAX_ROOTFS_ENTRIES {
                        f.error = Some(
                            "Expanded tree limit reached; collapse folders and refresh.".into(),
                        );
                        return None;
                    }
                    let selected = f.browser.selected_entry().map(|e| e.path.clone());
                    f.browser.nodes.insert(directory, entries);
                    f.browser.rebuild(selected.as_ref());
                }
                Ok(_) => {
                    f.error = Some("Rejected an out-of-scope or oversized directory result.".into())
                }
                Err(error) => f.error = Some(error),
            }
        }
        W::FileLoaded {
            session,
            root,
            path,
            generation,
            result,
        } => {
            if generation != app.workbench.file_request || app.workbench.active != Some(session) {
                return None;
            }
            let bytes = app.workbench.text_bytes();
            let s = app
                .workbench
                .sessions
                .iter_mut()
                .find(|s| s.id == session)?;
            if !s.folders.iter().any(|f| f.browser.root == root) || !path.starts_with(&root) {
                return None;
            }
            match result {
                Ok(content)
                    if content.len() <= TEXTAREA_MAX_BYTES
                        && bytes.saturating_add(content.len() * 2) <= WORKBENCH_MAX_TEXT
                        && s.tabs.len() < WORKBENCH_MAX_TABS =>
                {
                    if let Some(index) = s.tabs.iter().position(|t| t.path == path) {
                        s.tab = index;
                        return None;
                    }
                    let mut document = TextAreaState::new(content.clone());
                    document.cursor = 0;
                    document.set_mode(TextAreaMode::Insert);
                    s.tabs.push(WorkbenchTab {
                        reserved: content.len(),
                        saving: false,
                        viewport: WorkbenchViewport {
                            follow_cursor: true,
                            ..Default::default()
                        },
                        breakpoints: BTreeSet::new(),
                        root,
                        path,
                        document,
                        original: content,
                        error: None,
                        diff: None,
                    });
                    s.tab = s.tabs.len() - 1;
                    s.focus = WorkbenchFocus::Editor;
                    app.workbench.changed();
                }
                Ok(_) => {
                    app.notification =
                        Some("Workspace file or recovery buffer limit exceeded.".into())
                }
                Err(error) => {
                    app.notification = Some(format!("Could not open workspace file: {error}"))
                }
            }
        }
        W::Saved {
            session,
            path,
            content,
            result,
        } => {
            let s = app
                .workbench
                .sessions
                .iter_mut()
                .find(|s| s.id == session)?;
            let t = s.tabs.iter_mut().find(|t| t.path == path)?;
            t.saving = false;
            match result {
                Ok(()) => {
                    t.original = content;
                    t.error = None;
                    app.notification = Some(format!("Saved {}", path.display()));
                    app.workbench.changed();
                }
                Err(error) => {
                    t.error = Some(error.clone());
                    app.notification =
                        Some(format!("Workspace save failed; buffer retained: {error}"));
                }
            }
        }
        W::NewFileCreated {
            session,
            root,
            path,
            result,
        } => {
            if app.workbench.active != Some(session) {
                return None;
            }
            if let Err(error) = result {
                app.notification = Some(format!("Could not create workspace file: {error}"));
                return None;
            }
            app.workbench.file_request += 1;
            return Some(Effect::Workbench(WorkbenchRequest::Read {
                session,
                root,
                path,
                generation: app.workbench.file_request,
            }));
        }
        _ => unreachable!("Workbench result router"),
    }
    None
}

pub(super) fn release_terminal(app: &mut App) -> Option<Effect> {
    let session = app.workbench.current()?;
    if session.terminal_daemon != app.daemon.instance_id
        || session.terminal != app.selected_terminal_session().map(|s| s.id)
        || !app.selected_terminal_is_writer()
    {
        return None;
    }
    // Use the shared release state machine so a quick return waits for its epoch.
    update_with_workspace_authority(app, Action::TerminalReleaseControl)
}

pub(super) fn terminal(app: &mut App) -> Option<Effect> {
    let session = app.workbench.current()?;
    if session.terminal_daemon == app.daemon.instance_id
        && let Some(index) = session.terminal.and_then(|id| {
            app.daemon
                .pty_sessions
                .iter()
                .position(|pty| pty.id == id && !pty.lifecycle.is_terminal())
        })
    {
        app.select_terminal_pane(app.pane_layout.focused, index);
        app.terminal.browser_open = false;
        app.terminal.single_session = true;
        app.terminal.mode = TerminalWorkbenchMode::Live;
        let s = app.workbench.current_mut()?;
        s.focus = WorkbenchFocus::Terminal;
        s.output = WorkbenchOutput::Terminal;
        return update_with_workspace_authority(app, Action::TerminalTakeControl);
    }
    let root = session.folders.get(session.folder)?.browser.root.clone();
    let name = format!("Workbench {} · {}", session.id, session.name);
    let effect = update_with_workspace_authority(app, Action::TerminalCreateBuildShell);
    if let Some(Dialog::TerminalLaunch(dialog)) = app.active_dialog_mut() {
        dialog.request.cwd = root;
        dialog.request.name = name;
    }
    effect
}
