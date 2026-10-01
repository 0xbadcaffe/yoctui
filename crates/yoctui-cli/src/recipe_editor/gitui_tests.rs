use super::*;

fn open(app: &mut App, root: PathBuf) -> Effect {
    app.gitui_program = Some("/usr/bin/gitui".into());
    update(
        app,
        Action::OpenRecipeEditor {
            recipe: "Layer: fixture".into(),
            root,
            files: vec!["file.txt".into()],
        },
    );
    update(app, Action::OpenRecipeEditorGitUi).unwrap()
}

async fn drain(worker: &mut EditorGitUiIo, app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while worker.pending.is_some() {
        worker.poll(app).await;
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn editor_gitui_probes_real_repository_and_nonrepo_without_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repository with spaces");
    fs::create_dir(&repo).unwrap();
    assert!(
        std::process::Command::new("git")
            .args(["init", "-q"])
            .arg(&repo)
            .status()
            .unwrap()
            .success()
    );
    fs::write(repo.join("file.txt"), "unchanged\n").unwrap();
    let original = fs::read(repo.join("file.txt")).unwrap();
    let mut worker = EditorGitUiIo::default();
    let mut app = App::new(16, 4_096);
    worker.submit(open(&mut app, repo.clone()));
    drain(&mut worker, &mut app).await;
    assert!(
        matches!(app.active_dialog(), Some(Dialog::TerminalLaunch(dialog)) if dialog.request.cwd == repo)
    );
    assert_eq!(fs::read(repo.join("file.txt")).unwrap(), original);
    assert!(!repo.join(".git/index").exists());
    assert!(!repo.join(".git/index.lock").exists());
    let mut app = App::new(16, 4_096);
    worker.submit(open(&mut app, temp.path().into()));
    drain(&mut worker, &mut app).await;
    assert!(matches!(app.active_dialog(), Some(Dialog::RecipeEditor(_))));
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("not a readable Git worktree")
    );
}

#[tokio::test]
async fn editor_gitui_rejects_invalid_roots_symlinks_and_unrelated_repository() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let other = temp.path().join("other");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&other).unwrap();
    assert!(validate_editor_gitui_paths(&root, &root).await.is_ok());
    assert!(validate_editor_gitui_paths(&root, &other).await.is_err());
    assert!(
        validate_editor_gitui_paths(Path::new("relative"), &root)
            .await
            .is_err()
    );
    assert!(
        validate_editor_gitui_paths(&root.join("../root"), &root)
            .await
            .is_err()
    );
    fs::write(root.join("file"), "no").unwrap();
    assert!(
        validate_editor_gitui_paths(&root.join("file"), &root)
            .await
            .is_err()
    );
    assert!(
        validate_editor_gitui_paths(&root.join("missing"), &root)
            .await
            .is_err()
    );
    #[cfg(unix)]
    {
        let alias = temp.path().join("alias");
        std::os::unix::fs::symlink(&root, &alias).unwrap();
        assert!(validate_editor_gitui_paths(&alias, &alias).await.is_err());
        fs::create_dir(root.join("nested")).unwrap();
        assert!(
            validate_editor_gitui_paths(&alias.join("nested"), &alias)
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn editor_gitui_cancels_covered_and_replaced_requests_without_launching() {
    let temp = tempfile::tempdir().unwrap();
    let mut worker = EditorGitUiIo::default();
    let mut app = App::new(16, 4_096);
    worker.submit(open(&mut app, temp.path().into()));
    app.dialogs.push_front(Dialog::QuitConfirmation);
    assert!(worker.poll(&mut app).await);
    assert!(worker.pending.is_none());
    assert!(app.editor_gitui_pending.is_none());
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QuitConfirmation)
    ));
    app.dialogs.clear();
    worker.submit(open(&mut app, temp.path().into()));
    let old_generation = app.editor_gitui_generation;
    worker.submit(update(&mut app, Action::OpenRecipeEditorGitUi).unwrap());
    assert_ne!(old_generation, app.editor_gitui_generation);
    drain(&mut worker, &mut app).await;
    assert!(matches!(app.active_dialog(), Some(Dialog::RecipeEditor(_))));
}

#[tokio::test]
async fn editor_gitui_poll_remains_nonblocking_for_an_unresponsive_worker() {
    let temp = tempfile::tempdir().unwrap();
    let mut app = App::new(16, 4_096);
    open(&mut app, temp.path().into());
    let mut worker = EditorGitUiIo {
        pending: Some((
            app.editor_gitui_generation,
            temp.path().into(),
            tokio::spawn(std::future::pending()),
        )),
    };
    let changed = tokio::time::timeout(Duration::from_millis(50), worker.poll(&mut app))
        .await
        .unwrap();
    assert!(!changed);
    assert!(worker.pending.is_some());
    app.menu.open_application();
    assert!(worker.poll(&mut app).await);
    assert!(worker.pending.is_none());
}
