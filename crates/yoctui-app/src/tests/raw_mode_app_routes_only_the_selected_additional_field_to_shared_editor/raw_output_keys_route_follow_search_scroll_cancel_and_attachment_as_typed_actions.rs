use super::*;

#[test]
fn raw_output_keys_route_follow_search_scroll_cancel_and_attachment_as_typed_actions() {
    let state = raw_execution_state_fixture();
    let request = state.request.id.clone();
    let command = state.request.command.clone();
    let mut app = yoctui_model::App::new(16, 4096);
    app.raw_mode.execution_states.insert(request.clone(), state);
    app.raw_mode.execution = Some(command);
    app.raw_mode.view = yoctui_model::RawModeView::Execution;
    app.raw_mode.focus = yoctui_model::RawModeFocus::Execution;

    assert_eq!(
        raw_mode_input(&app, Input::Char('f')),
        Some(yoctui_model::RawModeAction::ToggleOutputFollow)
    );
    assert!(matches!(
        raw_mode_input(&app, Input::Up),
        Some(yoctui_model::RawModeAction::ScrollOutput { vertical: 1, .. })
    ));
    assert_eq!(
        raw_mode_input(&app, Input::Char('c')),
        Some(yoctui_model::RawModeAction::CancelExecution(
            request.clone()
        ))
    );
    let detach = raw_mode_input(&app, Input::Char('d')).unwrap();
    assert_eq!(
        yoctui_model::update(&mut app, yoctui_model::Action::RawMode(detach)),
        Some(yoctui_model::Effect::SetRawAttachment {
            request: request.clone(),
            attached: false,
        })
    );
    assert_eq!(
        raw_mode_input(&app, Input::Char('/')),
        Some(yoctui_model::RawModeAction::BeginOutputSearch)
    );
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::RawMode(yoctui_model::RawModeAction::BeginOutputSearch),
    );
    assert_eq!(
        raw_mode_input(&app, Input::Char('界')),
        Some(yoctui_model::RawModeAction::AppendOutputSearch('界'))
    );
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::RawMode(yoctui_model::RawModeAction::FinishOutputSearch),
    );
    assert_eq!(
        raw_mode_input(&app, Input::Esc),
        Some(yoctui_model::RawModeAction::CloseExecution)
    );
    assert_eq!(
        yoctui_model::update(
            &mut app,
            yoctui_model::Action::RawMode(yoctui_model::RawModeAction::CloseExecution),
        ),
        Some(yoctui_model::Effect::SetRawAttachment {
            request,
            attached: false,
        })
    );
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Browser);
}

#[test]
fn navigator_raw_destination_detaches_running_execution_and_opens_the_catalog() {
    let state = raw_execution_state_fixture();
    let request = state.request.id.clone();
    let command = state.request.command.clone();
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = yoctui_model::Screen::Dashboard;
    app.raw_mode.execution_states.insert(request.clone(), state);
    app.raw_mode.execution = Some(command);
    app.raw_mode.view = yoctui_model::RawModeView::Execution;
    app.raw_mode.focus = yoctui_model::RawModeFocus::Execution;

    assert_eq!(
        yoctui_model::update(
            &mut app,
            yoctui_model::Action::Open(yoctui_model::Screen::RawMode),
        ),
        Some(yoctui_model::Effect::SetRawAttachment {
            request,
            attached: false,
        })
    );
    assert_eq!(app.screen, yoctui_model::Screen::RawMode);
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Browser);
    assert_eq!(
        app.raw_mode.browser_column,
        yoctui_model::RawBrowserColumn::Commands
    );
}
