use super::*;

#[test]
fn git_status_reducer_preserves_independent_dirty_and_sync_facts() {
    let mut app = crate::App::new(10, 1024);
    crate::update(
        &mut app,
        crate::Action::SourceGitStatusUpdated(SourceGitStatus::Ready(SourceGitSummary {
            upstream: Some("origin/master".into()),
            staged: 2,
            unstaged: 1,
            ahead: 3,
            ..Default::default()
        })),
    );
    let label = app.source_git_status.label().unwrap();
    assert!(label.contains("+2 ~1 ahead 3"));
    assert!(!label.contains("synced"));
}
