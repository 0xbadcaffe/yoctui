use super::*;

#[tokio::test]
async fn git_status_real_worktree_reports_dirty_tracking_and_missing_source() {
    let root = std::env::temp_dir().join(format!("yoctui-git-status-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-b", "master"]);
    std::fs::write(root.join("tracked"), "first\n").unwrap();
    git(&root, &["add", "tracked"]);
    git(&root, &["commit", "-m", "first"]);
    git(&root, &["branch", "upstream"]);
    git(&root, &["branch", "--set-upstream-to=upstream"]);
    let SourceGitStatus::Ready(clean) = inspect_source_git(&root).await else {
        panic!("expected status");
    };
    assert_eq!(
        (clean.ahead, clean.behind, clean.staged, clean.unstaged),
        (0, 0, 0, 0)
    );
    std::fs::write(root.join("tracked"), "second\n").unwrap();
    git(&root, &["commit", "-am", "second"]);
    let SourceGitStatus::Ready(ahead) = inspect_source_git(&root).await else {
        panic!();
    };
    assert_eq!((ahead.ahead, ahead.behind), (1, 0));
    git(&root, &["checkout", "upstream"]);
    git(&root, &["branch", "--set-upstream-to=master"]);
    let SourceGitStatus::Ready(behind) = inspect_source_git(&root).await else {
        panic!();
    };
    assert_eq!((behind.ahead, behind.behind), (0, 1));
    git(&root, &["branch", "--unset-upstream"]);
    std::fs::write(root.join("tracked"), "staged\n").unwrap();
    git(&root, &["add", "tracked"]);
    std::fs::write(root.join("tracked"), "unstaged\n").unwrap();
    std::fs::write(root.join("new\nfile"), "new").unwrap();
    let SourceGitStatus::Ready(dirty) = inspect_source_git(&root).await else {
        panic!();
    };
    assert_eq!((dirty.staged, dirty.unstaged, dirty.untracked), (1, 1, 1));
    assert!(dirty.upstream.is_none());
    std::fs::remove_dir_all(&root).unwrap();
    assert!(matches!(
        inspect_source_git(&root).await,
        SourceGitStatus::Unavailable(_)
    ));
}
