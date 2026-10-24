use std::path::{Path, PathBuf};
use yoctui_model::*;

fn send(app: &mut App, action: WorkbenchAction) -> Option<Effect> {
    update(app, Action::Workbench(action))
}
fn fixture() -> App {
    let mut app = App::new(32, 8192);
    app.workspace.source_dir = Some("/source".into());
    send(
        &mut app,
        WorkbenchAction::OpenRecipe {
            recipe: "busybox".into(),
            root: "/source".into(),
        },
    );
    let effect = send(
        &mut app,
        WorkbenchAction::Form(WorkbenchFormKind::AddFolder),
    );
    assert!(matches!(
        effect,
        Some(Effect::Workbench(WorkbenchRequest::BrowseFolders { .. }))
    ));
    loaded(&mut app, "/source", &["/source/poky", "/source/scripts"]);
    app
}
fn loaded(app: &mut App, directory: &str, entries: &[&str]) {
    let generation = app.workbench.folder_browser.as_ref().unwrap().generation;
    send(
        app,
        WorkbenchAction::FoldersBrowsed {
            generation,
            result: Ok((
                directory.into(),
                entries.iter().map(PathBuf::from).collect(),
            )),
        },
    );
}
#[test]
fn folder_browser_navigates_parent_child_and_adds_the_selected_directory() {
    let mut app = fixture();
    send(&mut app, WorkbenchAction::BrowseFolders);
    send(&mut app, WorkbenchAction::BrowseSelect(1));
    assert!(matches!(send(&mut app, WorkbenchAction::BrowseOpen),
        Some(Effect::Workbench(WorkbenchRequest::BrowseFolders { path, .. })) if path == Path::new("/source/poky")));
    loaded(&mut app, "/source/poky", &[]);
    assert!(matches!(send(&mut app, WorkbenchAction::BrowseParent),
        Some(Effect::Workbench(WorkbenchRequest::BrowseFolders { path, .. })) if path == Path::new("/source")));
    loaded(&mut app, "/source", &["/source/poky", "/source/scripts"]);
    send(&mut app, WorkbenchAction::BrowseSelect(2));
    assert!(matches!(send(&mut app, WorkbenchAction::BrowseChoose),
        Some(Effect::Workbench(WorkbenchRequest::Folder { path, .. })) if path == Path::new("/source/scripts")));
    assert!(app.workbench.form.is_none());
    assert!(app.workbench.folder_browser.is_none());
    // Adding still goes through the existing canonicalization/FolderLoaded boundary.
    assert_eq!(app.workbench.current().unwrap().folders.len(), 1);
}
#[test]
fn folder_browser_ignores_stale_and_cancelled_directory_results() {
    let mut app = fixture();
    let old = app.workbench.folder_browser.as_ref().unwrap().generation;
    send(&mut app, WorkbenchAction::BrowseParent);
    send(
        &mut app,
        WorkbenchAction::FoldersBrowsed {
            generation: old,
            result: Ok(("/wrong".into(), vec![])),
        },
    );
    assert_eq!(
        app.workbench.folder_browser.as_ref().unwrap().directory,
        PathBuf::from("/")
    );
    loaded(&mut app, "/", &[]);
    assert!(send(&mut app, WorkbenchAction::BrowseParent).is_none());
    send(&mut app, WorkbenchAction::Cancel);
    send(
        &mut app,
        WorkbenchAction::FoldersBrowsed {
            generation: old,
            result: Err("late error".into()),
        },
    );
    assert!(app.workbench.folder_browser.is_none());
    assert!(app.workbench.form.is_none());
}
#[test]
fn folder_browser_keeps_typed_paths_and_refuses_loading_or_failed_selections() {
    let mut app = fixture();
    send(&mut app, WorkbenchAction::FormClear);
    for c in "/other folder".chars() {
        send(&mut app, WorkbenchAction::FormText(c));
    }
    send(&mut app, WorkbenchAction::BrowseFolders);
    assert!(send(&mut app, WorkbenchAction::BrowseChoose).is_none());
    let generation = app.workbench.folder_browser.as_ref().unwrap().generation;
    send(
        &mut app,
        WorkbenchAction::FoldersBrowsed {
            generation,
            result: Err("Permission denied".into()),
        },
    );
    assert!(send(&mut app, WorkbenchAction::BrowseChoose).is_none());
    assert_eq!(app.workbench.form.as_ref().unwrap().text, "/other folder");
    send(&mut app, WorkbenchAction::BrowseFolders);
    assert!(matches!(send(&mut app, WorkbenchAction::ConfirmForm),
        Some(Effect::Workbench(WorkbenchRequest::Folder { path, .. })) if path == Path::new("/other folder")));
}
