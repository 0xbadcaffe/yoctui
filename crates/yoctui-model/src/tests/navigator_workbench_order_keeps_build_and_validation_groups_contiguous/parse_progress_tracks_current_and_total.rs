use super::*;

#[test]
fn parse_progress_tracks_current_and_total() {
    let mut app = App::new(10, 1_000);
    app.build.status = BuildStatus::LoadingWorkspace;
    let _ = update(
        &mut app,
        Action::ParseProgress {
            current: Some(8),
            total: Some(20),
        },
    );
    assert_eq!(app.build.status, BuildStatus::Parsing);
    assert_eq!(app.build.parse_current, Some(8));
    assert_eq!(app.build.parse_total, Some(20));
    let _ = update(&mut app, Action::BuildStarted);
    assert_eq!(app.build.parse_current, None);
    assert_eq!(app.build.parse_total, None);
}
