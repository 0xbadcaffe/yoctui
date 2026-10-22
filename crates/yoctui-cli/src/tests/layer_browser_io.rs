use super::*;
use std::{fs, time::Duration};

async fn finish(io: &mut LayerBrowserIo, app: &mut App) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while io.directory.is_some() || io.preview.is_some() {
            io.poll(app);
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("browsing worker did not finish");
}

fn fixture() -> (tempfile::TempDir, App, PathBuf) {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let mut app = App::new(8, 1024);
    app.onboarding.open = false;
    app.screen = Screen::Layers;
    (temporary, app, root)
}

#[tokio::test]
async fn browser_io_does_not_wait_for_io_and_cancels_when_navigating_away() {
    let (_temporary, mut app, root) = fixture();
    let mut io = LayerBrowserIo::default();
    let held = Arc::clone(&io.workers).acquire_many_owned(2).await.unwrap();
    io.directory(
        &mut app,
        "Build directory".into(),
        root.clone(),
        root.clone(),
    );
    assert!(io.directory.is_some());
    app.screen = Screen::Dashboard;
    assert!(!io.poll(&mut app));
    assert!(io.directory.is_none());
    drop(held);
    tokio::task::yield_now().await;
    assert!(app.layer_browser.as_ref().unwrap().entries.is_empty());
}

#[tokio::test]
async fn browser_io_rapid_changes_keep_only_the_latest_request() {
    let (_temporary, mut app, root) = fixture();
    let mut io = LayerBrowserIo::default();
    let held = Arc::clone(&io.workers).acquire_many_owned(2).await.unwrap();
    for i in 0..20 {
        let child = root.join(format!("folder-{i}"));
        fs::create_dir(&child).unwrap();
        fs::write(child.join(format!("file-{i}")), format!("content-{i}")).unwrap();
        io.directory(&mut app, "Build directory".into(), child.clone(), child);
    }
    drop(held);
    finish(&mut io, &mut app).await;
    let browser = app.layer_browser.as_ref().unwrap();
    assert!(browser.root.ends_with("folder-19"));
    assert_eq!(browser.entries.len(), 1);
    assert_eq!(browser.preview, "content-19");
    assert!(browser.incomplete.is_empty());
    assert_eq!(io.workers.available_permits(), 2);
}

#[tokio::test]
async fn browser_io_streams_large_directories_and_releases_cancelled_workers() {
    let (_temporary, mut app, root) = fixture();
    for i in 0..1500 {
        fs::write(root.join(format!("file-{i:04}")), "test").unwrap();
    }
    let mut io = LayerBrowserIo::default();
    io.directory(
        &mut app,
        "Build directory".into(),
        root.clone(),
        root.clone(),
    );
    tokio::time::timeout(Duration::from_secs(3), async {
        while io.directory.as_ref().unwrap().results.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(io.poll(&mut app));
    let browser = app.layer_browser.as_ref().unwrap();
    assert!(!browser.entries.is_empty());
    assert!(browser.entries.len() < 1500);
    assert!(browser.incomplete.contains(&root));
    app.layer_browser = None;
    io.poll(&mut app);
    let returned = tokio::time::timeout(
        Duration::from_secs(3),
        Arc::clone(&io.workers).acquire_many_owned(2),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(app.layer_browser.is_none());
    drop(returned);
}

#[tokio::test]
async fn browser_io_final_directory_is_complete_sorted_and_fresh_on_refresh() {
    let (_temporary, mut app, root) = fixture();
    fs::write(root.join("z.txt"), "z").unwrap();
    fs::write(root.join("a.txt"), "a").unwrap();
    let mut io = LayerBrowserIo::default();
    io.directory(
        &mut app,
        "Build directory".into(),
        root.clone(),
        root.clone(),
    );
    finish(&mut io, &mut app).await;
    let browser = app.layer_browser.as_ref().unwrap();
    assert_eq!(browser.entries[0].path, root.join("a.txt"));
    assert!(browser.incomplete.is_empty());
    fs::write(root.join("new.txt"), "new").unwrap();
    io.directory(
        &mut app,
        "Build directory".into(),
        root.clone(),
        root.clone(),
    );
    finish(&mut io, &mut app).await;
    assert_eq!(app.layer_browser.as_ref().unwrap().entries.len(), 3);
}

#[tokio::test]
async fn browser_io_preview_cannot_replace_a_newer_selection() {
    let (_temporary, mut app, root) = fixture();
    fs::write(root.join("a.txt"), "old").unwrap();
    fs::write(root.join("b.txt"), "new").unwrap();
    let mut io = LayerBrowserIo::default();
    io.directory(
        &mut app,
        "Build directory".into(),
        root.clone(),
        root.clone(),
    );
    finish(&mut io, &mut app).await;
    let held = Arc::clone(&io.workers).acquire_many_owned(2).await.unwrap();
    io.preview(&mut app, root.join("a.txt"));
    assert!(app.layer_browser.as_ref().unwrap().preview.is_empty());
    app.layer_browser.as_mut().unwrap().selection = 1;
    io.poll(&mut app);
    assert!(io.preview.is_none());
    drop(held);
    io.preview(&mut app, root.join("b.txt"));
    finish(&mut io, &mut app).await;
    assert_eq!(app.layer_browser.as_ref().unwrap().preview, "new");
}

#[tokio::test]
async fn browser_io_rejects_results_after_build_context_changes() {
    let (_temporary, mut app, root) = fixture();
    let mut io = LayerBrowserIo::default();
    let held = Arc::clone(&io.workers).acquire_many_owned(2).await.unwrap();
    io.directory(
        &mut app,
        "Build directory".into(),
        root.clone(),
        root.clone(),
    );
    app.workspace.build_dir = Some(root.join("other-build"));
    io.poll(&mut app);
    assert!(io.directory.is_none());
    drop(held);
    assert!(app.layer_browser.as_ref().unwrap().entries.is_empty());
}

#[test]
fn browser_io_incomplete_child_is_not_treated_as_a_finished_cache_hit() {
    let (_temporary, mut app, root) = fixture();
    let child = root.join("child");
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "layer".into(),
            root: root.clone(),
            directory: root.clone(),
            entries: vec![LayerBrowserEntry {
                path: child.clone(),
                is_dir: true,
                ..Default::default()
            }],
        },
    );
    let browser = app.layer_browser.as_mut().unwrap();
    browser.nodes.insert(child.clone(), Vec::new());
    browser.incomplete.insert(child.clone());
    browser.expanded.insert(child.clone());
    assert!(
        matches!(update(&mut app, Action::LayerBrowserExpand), Some(Effect::LoadLayerBrowserDirectory { directory, .. }) if directory == child)
    );
}

#[tokio::test]
async fn browser_io_collapsed_partial_directory_is_not_reopened_by_late_results() {
    let (_temporary, mut app, root) = fixture();
    let child = root.join("child");
    fs::create_dir(&child).unwrap();
    for i in 0..1500 {
        fs::write(child.join(format!("file-{i}")), "data").unwrap();
    }
    let mut io = LayerBrowserIo::default();
    io.directory(
        &mut app,
        "Build directory".into(),
        root.clone(),
        root.clone(),
    );
    finish(&mut io, &mut app).await;
    io.directory(&mut app, "Build directory".into(), root, child.clone());
    tokio::time::timeout(Duration::from_secs(3), async {
        while io.directory.as_ref().unwrap().results.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    io.poll(&mut app);
    assert!(
        app.layer_browser
            .as_ref()
            .unwrap()
            .expanded
            .contains(&child)
    );
    let _ = update(&mut app, Action::LayerBrowserEnter);
    assert!(
        !app.layer_browser
            .as_ref()
            .unwrap()
            .expanded
            .contains(&child)
    );
    io.poll(&mut app);
    assert!(io.directory.is_none());
    assert!(
        app.layer_browser
            .as_ref()
            .unwrap()
            .incomplete
            .contains(&child)
    );
    assert!(
        matches!(update(&mut app, Action::LayerBrowserExpand), Some(Effect::LoadLayerBrowserDirectory { directory, .. }) if directory == child)
    );
}

#[cfg(unix)]
#[tokio::test]
async fn browser_io_rootfs_stream_preserves_target_metadata_and_rejects_symlink_preview() {
    use std::os::unix::fs::symlink;
    let (_temporary, mut app, root) = fixture();
    fs::create_dir(root.join("etc")).unwrap();
    fs::write(root.join("etc/passwd"), "root:x:0:0:root:/root:/bin/sh\n").unwrap();
    fs::write(root.join("regular"), "target").unwrap();
    symlink("regular", root.join("link")).unwrap();
    let mut io = LayerBrowserIo::default();
    io.directory(
        &mut app,
        "Rootfs: fixture".into(),
        root.clone(),
        root.clone(),
    );
    finish(&mut io, &mut app).await;
    let browser = app.layer_browser.as_mut().unwrap();
    let index = browser
        .entries
        .iter()
        .position(|entry| entry.path == root.join("link"))
        .unwrap();
    assert!(!browser.entries[index].is_dir);
    assert_eq!(
        browser.entries[index]
            .rootfs_metadata
            .as_ref()
            .unwrap()
            .kind,
        yoctui_model::RootfsEntryKind::Symlink
    );
    browser.selection = index;
    io.preview(&mut app, root.join("link"));
    finish(&mut io, &mut app).await;
    assert!(app.notification.as_ref().unwrap().contains("symlink"));
    assert_ne!(app.layer_browser.as_ref().unwrap().preview, "target");
}

#[cfg(unix)]
#[tokio::test]
async fn browser_io_special_file_preview_does_not_block_the_pool() {
    use std::os::unix::ffi::OsStrExt;
    let (_temporary, mut app, root) = fixture();
    let fifo = root.join("fifo");
    let path = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let mut io = LayerBrowserIo::default();
    io.directory(&mut app, "Build directory".into(), root.clone(), root);
    finish(&mut io, &mut app).await;
    assert!(app.notification.as_ref().unwrap().contains("regular files"));
    assert_eq!(io.workers.available_permits(), 2);
}

#[tokio::test]
async fn browser_io_reports_missing_directories_without_reopening_closed_browsers() {
    let (_temporary, mut app, root) = fixture();
    let mut io = LayerBrowserIo::default();
    io.directory(
        &mut app,
        "Build directory".into(),
        root.clone(),
        root.join("missing"),
    );
    finish(&mut io, &mut app).await;
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("Could not read layer directory")
    );
}
