use super::*;

#[test]
fn focus_routing_passes_unmatched_global_and_workspace_keys_onward() {
    use yoctui_model::FocusTarget;

    for focus in [
        FocusTarget::Navigator,
        FocusTarget::Workspace,
        FocusTarget::Inspector,
    ] {
        let mut app = App::new(10, 1_000);
        app.focus = focus;
        for input in [Input::CtrlP, Input::Char('?'), Input::F5, Input::Char('r')] {
            assert!(pane_focus_route(&app, input).is_none());
        }
        assert!(matches!(
            pane_focus_route(&app, Input::Tab),
            Some(Action::CycleFocus { backwards: false })
        ));
        assert!(matches!(
            pane_focus_route(&app, Input::Char('q')),
            Some(Action::Quit)
        ));
    }
    let mut navigator = App::new(10, 1_000);
    navigator.focus = FocusTarget::Navigator;
    assert!(matches!(
        pane_focus_route(&navigator, Input::Down),
        Some(Action::SelectNavigator { delta: 1 })
    ));
    let mut workspace = App::new(10, 1_000);
    workspace.focus = FocusTarget::Workspace;
    assert!(pane_focus_route(&workspace, Input::Down).is_none());
}
