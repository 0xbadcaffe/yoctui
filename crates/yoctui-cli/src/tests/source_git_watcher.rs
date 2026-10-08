use super::*;

#[tokio::test]
async fn slow_watch_setup_never_blocks_poll_and_is_cancelled_on_workspace_change() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut poller = SourceGitPoller::default();
    poller.source = Some(PathBuf::from("/old-source"));
    poller.watch_setup = Some(WatchSetup {
        cancelled: cancelled.clone(),
        task: tokio::spawn(std::future::pending()),
    });
    poller.next = Some(Instant::now() + FALLBACK_REFRESH);
    let mut app = App::new(10, 1000);
    app.workspace.source_dir = Some(PathBuf::from("/old-source"));
    let before = Instant::now();
    poller.poll(&mut app).await;
    assert!(before.elapsed() < Duration::from_millis(100));
    assert!(poller.watch_setup.is_some());
    assert!(!cancelled.load(Ordering::Relaxed));
    app.workspace.source_dir = None;
    poller.poll(&mut app).await;
    assert!(cancelled.load(Ordering::Relaxed));
    assert!(poller.watch_setup.is_none());
    assert!(poller.watcher.is_none());
}

#[tokio::test]
async fn non_git_source_falls_back_without_a_watcher() {
    let directory = std::env::temp_dir().join(format!("yoctui-non-git-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    assert!(
        WatchSetup::spawn(directory.clone(), None)
            .finish()
            .await
            .is_none()
    );
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn tracked_watch_plan_excludes_build_output_and_path_escape() {
    let source = Path::new("/source");
    let plan = tracked_directories(source, Some(Path::new("/source/build/romulus")),
        b"meta/recipes/a.bb\0meta/classes/base.bbclass\0README\0build/romulus/conf/local.conf\0../escape/file\0/absolute/file\0");
    assert_eq!(
        plan,
        BTreeSet::from([
            source.into(),
            "/source/meta".into(),
            "/source/meta/recipes".into(),
            "/source/meta/classes".into()
        ])
    );
    assert!(!plan.iter().any(|path| path.starts_with("/source/build")));
}

#[test]
fn tracked_watch_plan_is_bounded() {
    let tracked = (0..MAX_WATCH_DIRECTORIES * 2)
        .map(|n| format!("dir{n}/file\0"))
        .collect::<String>();
    assert_eq!(
        tracked_directories(Path::new("/source"), None, tracked.as_bytes()).len(),
        MAX_WATCH_DIRECTORIES
    );
}

#[test]
fn cancelled_watch_install_does_not_register_anything() {
    assert!(
        install(
            BTreeSet::from([PathBuf::from("/")]),
            Arc::new(AtomicBool::new(true))
        )
        .is_none()
    );
}
