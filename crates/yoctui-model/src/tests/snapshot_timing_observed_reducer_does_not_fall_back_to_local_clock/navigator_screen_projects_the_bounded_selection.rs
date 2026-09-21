use super::*;

#[test]
fn navigator_screen_projects_the_bounded_selection() {
    let mut app = App::new(10, 1_000);
    app.navigator_selection = 2;
    assert_eq!(app.navigator_screen(), Screen::Layers);
    app.navigator_selection = usize::MAX;
    assert_eq!(app.navigator_screen(), Screen::Dashboard);
}
