use super::*;

fn git(root: &Path, arguments: &[&str]) {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .status()
        .unwrap();
    assert!(status.success());
}

#[tokio::test]
async fn source_git_refreshes_after_a_worktree_event() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-source-watch-{}-{}",
        std::process::id(),
        yoctui_utils::unix_ms()
    ));
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.name", "Yoctui Test"]);
    git(&root, &["config", "user.email", "test@example.invalid"]);
    std::fs::create_dir_all(root.join("meta/recipes")).unwrap();
    std::fs::create_dir_all(root.join("build/deep/generated")).unwrap();
    std::fs::write(root.join(".gitignore"), "build/\n").unwrap();
    std::fs::write(root.join("meta/recipes/tracked.txt"), "clean\n").unwrap();
    git(&root, &["add", ".gitignore", "meta/recipes/tracked.txt"]);
    git(&root, &["commit", "-qm", "initial"]);

    let mut app = App::new(10, 1_000);
    app.workspace.source_dir = Some(root.clone());
    let mut poller = SourceGitPoller::default();
    let deadline = Instant::now() + Duration::from_secs(5);
    while poller.watch_setup.is_some()
        || poller.pending.is_some()
        || poller.next.is_none_or(|next| next <= Instant::now())
        || !matches!(
            app.source_git_status,
            yoctui_model::SourceGitStatus::Ready(_)
        )
    {
        poller.poll(&mut app).await;
        assert!(Instant::now() < deadline, "initial Git status timed out");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // A generated sibling tree is not watched even without a selected build.
    std::fs::write(root.join("build/deep/generated/output"), "generated\n").unwrap();
    let quiet_deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < quiet_deadline {
        assert!(
            !poller.poll(&mut app).await,
            "a read-only Git status probe must not trigger itself"
        );
        assert!(matches!(
            app.source_git_status,
            yoctui_model::SourceGitStatus::Ready(_)
        ));
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    poller.next = Some(Instant::now());
    assert!(poller.poll(&mut app).await);
    assert!(
        matches!(
            app.source_git_status,
            yoctui_model::SourceGitStatus::Ready(_)
        ),
        "a background refresh must keep the last known Git status visible"
    );
    while poller.pending.is_some() {
        poller.poll(&mut app).await;
        assert!(matches!(
            app.source_git_status,
            yoctui_model::SourceGitStatus::Ready(_)
        ));
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    std::fs::write(root.join("meta/recipes/tracked.txt"), "dirty\n").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        poller.poll(&mut app).await;
        if matches!(
            &app.source_git_status,
            yoctui_model::SourceGitStatus::Ready(summary) if summary.unstaged == 1
        ) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "worktree event did not refresh Git status"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    git(&root, &["add", "meta/recipes/tracked.txt"]);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        poller.poll(&mut app).await;
        if matches!(
            &app.source_git_status,
            yoctui_model::SourceGitStatus::Ready(summary) if summary.staged == 1 && summary.unstaged == 0
        ) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Git index event did not refresh status"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    drop(poller);
    std::fs::remove_dir_all(root).unwrap();
}
