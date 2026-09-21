use super::*;

#[test]
fn archive_navigation_never_changes_live_build() {
    let mut app = App::new(32, 4096);
    let live = app.build.clone();
    reduce_saved_build(&mut app, SavedBuildAction::Select(-100));
    reduce_saved_build(&mut app, SavedBuildAction::Open);
    assert!(app.saved_builds.view.is_none());
    reduce_saved_build(&mut app, SavedBuildAction::ShiftView(-1));
    assert_eq!(app.saved_builds.view, Some(SavedBuildView::Errors));
    reduce_saved_build(&mut app, SavedBuildAction::Scroll(isize::MAX));
    assert_eq!(app.saved_builds.scroll, 4096);
    reduce_saved_build(&mut app, SavedBuildAction::Close);
    assert_eq!(app.build, live);
}
