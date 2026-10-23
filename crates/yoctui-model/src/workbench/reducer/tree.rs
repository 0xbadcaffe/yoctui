use super::*;

pub(super) fn directory_request(app: &mut App, directory: Option<PathBuf>) -> Option<Effect> {
    let s = app.workbench.current_mut()?;
    let f = s.folders.get_mut(s.folder)?;
    let directory = directory.unwrap_or_else(|| f.browser.root.clone());
    if f.requests.len() >= 32 && !f.requests.contains_key(&directory) {
        app.notification =
            Some("Wait for pending folder scans before expanding more directories.".into());
        return None;
    }
    f.request = f.request.wrapping_add(1);
    f.requests.insert(directory.clone(), f.request);
    f.loading = true;
    f.error = None;
    Some(Effect::Workbench(WorkbenchRequest::Directory {
        session: s.id,
        root: f.browser.root.clone(),
        directory,
        generation: f.request,
    }))
}

pub(super) fn open_entry(app: &mut App) -> Option<Effect> {
    let s = app.workbench.current_mut()?;
    let f = s.folders.get_mut(s.folder)?;
    let entry = f.browser.selected_entry()?.clone();
    if entry.is_dir {
        if f.browser.expanded.remove(&entry.path) {
            f.browser.rebuild(Some(&entry.path));
            return None;
        }
        f.browser.expanded.insert(entry.path.clone());
        if f.browser.nodes.contains_key(&entry.path) {
            f.browser.rebuild(Some(&entry.path));
            return None;
        }
        return directory_request(app, Some(entry.path));
    }
    if let Some(index) = s.tabs.iter().position(|t| t.path == entry.path) {
        s.tab = index;
        s.focus = WorkbenchFocus::Editor;
        return None;
    }
    if s.tabs.len() >= WORKBENCH_MAX_TABS {
        app.notification =
            Some("Close an editor tab before opening another (16-tab limit).".into());
        return None;
    }
    let (session, root, path) = (s.id, f.browser.root.clone(), entry.path);
    app.workbench.file_request += 1;
    Some(Effect::Workbench(WorkbenchRequest::Read {
        session,
        root,
        path,
        generation: app.workbench.file_request,
    }))
}
