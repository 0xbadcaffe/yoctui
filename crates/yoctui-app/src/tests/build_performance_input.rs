use super::*;
#[test]
fn performance_keys_require_the_visible_focused_workspace() {
    let mut app = App::new(100, 4096);
    assert_eq!(build_performance_action(&app, Input::Char('b')), None);
    app.screen = Screen::BuildPerformance;
    assert_eq!(build_performance_action(&app, Input::Char('b')), None);
    app.focus = FocusTarget::Workspace;
    assert_eq!(
        build_performance_action(&app, Input::Char('b')),
        Some(Action::BuildPerformance(P::Balanced))
    );
    assert_eq!(build_performance_action(&app, Input::Tab), None);
    assert_eq!(build_performance_action(&app, Input::Char('/')), None);
    assert_eq!(
        build_performance_action(&app, Input::Esc),
        Some(Action::Focus(FocusTarget::Navigator))
    );
}
