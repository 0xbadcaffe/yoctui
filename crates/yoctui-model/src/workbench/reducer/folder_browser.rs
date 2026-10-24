use super::*;

pub(super) fn start(app: &mut App, editing_path: bool) -> Option<Effect> {
    let form = app.workbench.form.as_ref()?;
    if form.kind != WorkbenchFormKind::AddFolder {
        return None;
    }
    let path = PathBuf::from(form.text.trim());
    if !path.is_absolute() {
        app.notification = Some("Enter an absolute folder path before browsing.".into());
        return None;
    }
    app.workbench.generation = app.workbench.generation.saturating_add(1);
    let generation = app.workbench.generation;
    app.workbench.folder_browser = Some(WorkbenchFolderBrowser {
        directory: path.clone(),
        entries: Vec::new(),
        selection: 0,
        generation,
        loading: true,
        error: None,
        editing_path,
    });
    Some(Effect::Workbench(WorkbenchRequest::BrowseFolders {
        path,
        generation,
    }))
}

pub(super) fn reduce(app: &mut App, action: WorkbenchAction) -> Option<Effect> {
    use WorkbenchAction as W;
    if !app
        .workbench
        .form
        .as_ref()
        .is_some_and(|f| f.kind == WorkbenchFormKind::AddFolder)
    {
        return None;
    }
    match action {
        W::BrowseFolders => {
            if let Some(browser) = &mut app.workbench.folder_browser {
                if !browser.editing_path {
                    browser.editing_path = true;
                    return None;
                }
                if app.workbench.form.as_ref()?.text.trim() == browser.directory.to_string_lossy()
                    && browser.error.is_none()
                {
                    browser.editing_path = false;
                    return None;
                }
            }
            return start(app, false);
        }
        W::BrowseSelect(delta) => {
            let b = app.workbench.folder_browser.as_mut()?;
            b.editing_path = false;
            b.selection = shifted_index(b.selection, delta, b.entries.len() + 1);
            app.workbench.form.as_mut()?.text = b.directory.display().to_string();
        }
        W::BrowseSelectAt(index) => {
            let b = app.workbench.folder_browser.as_mut()?;
            b.editing_path = false;
            b.selection = index.min(b.entries.len());
            app.workbench.form.as_mut()?.text = b.directory.display().to_string();
        }
        W::BrowseParent => {
            let b = app.workbench.folder_browser.as_ref()?;
            let parent = b.directory.parent()?.to_path_buf();
            app.workbench.form.as_mut()?.text = parent.display().to_string();
            return start(app, false);
        }
        W::BrowseOpen | W::BrowseChoose => {
            let b = app.workbench.folder_browser.as_ref()?;
            if b.loading || b.error.is_some() {
                return None;
            }
            let choose = action == W::BrowseChoose || b.selection == 0;
            app.workbench.form.as_mut()?.text = b.selected_path().display().to_string();
            if choose {
                return forms::confirm_form(app);
            }
            return start(app, false);
        }
        W::FoldersBrowsed { generation, result } => {
            let b = app.workbench.folder_browser.as_mut()?;
            if b.generation != generation {
                return None;
            }
            b.loading = false;
            match result {
                Ok((directory, entries)) => {
                    b.directory = directory;
                    b.entries = entries;
                    b.selection = b.selection.min(b.entries.len());
                    if !b.editing_path {
                        app.workbench.form.as_mut()?.text = b.directory.display().to_string();
                    }
                }
                Err(error) => b.error = Some(error),
            }
        }
        _ => {}
    }
    None
}
