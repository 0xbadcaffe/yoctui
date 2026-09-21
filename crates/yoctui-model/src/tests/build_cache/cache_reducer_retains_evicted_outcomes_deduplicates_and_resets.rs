use super::*;

#[test]
fn cache_reducer_retains_evicted_outcomes_deduplicates_and_resets() {
    use crate::{Action, App, TaskId};
    let mut app = App::new(16, 4096);
    for index in 0..crate::MAX_COMPLETED_TASKS + 5 {
        let id = TaskId(format!("recipe-{index}:do_fetch"));
        crate::update(
            &mut app,
            Action::TaskCompleted {
                id: id.clone(),
                success: true,
            },
        );
        crate::update(&mut app, Action::TaskCompleted { id, success: true });
    }
    assert_eq!(
        app.build.cache.fetch_completed,
        crate::MAX_COMPLETED_TASKS + 5
    );
    assert_eq!(
        app.overview_cache().fetch_completed,
        crate::MAX_COMPLETED_TASKS + 5
    );
    assert!(app.cache_status_lines()[2].contains("unverified"));
    app.workspace
        .variables
        .insert("BB_NO_NETWORK".into(), "1".into());
    assert!(app.cache_status_lines()[2].contains("disabled"));
    crate::update(
        &mut app,
        Action::BuildRequested {
            target: Some("image".into()),
        },
    );
    assert_eq!(app.build.cache, BuildCacheState::default());
}
