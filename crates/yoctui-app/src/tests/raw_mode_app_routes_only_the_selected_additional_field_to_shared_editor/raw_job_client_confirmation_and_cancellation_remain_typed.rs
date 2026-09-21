use super::*;

#[test]
fn raw_job_client_confirmation_and_cancellation_remain_typed() {
    let mut preview_state = yoctui_model::RawModeState::new(yoctui_model::builtin_raw_catalog());
    preview_state.view = yoctui_model::RawModeView::Preview;
    assert_eq!(
        raw_mode_input(&preview_state, Input::Enter),
        Some(yoctui_model::RawModeAction::ConfirmPreview)
    );

    let state = raw_execution_state_fixture();
    let wire = raw_execution_request_to_protocol(&state.request).unwrap();
    assert_eq!(wire.request_id, state.request.id.as_str());
    assert_eq!(wire.command_id, state.request.command.as_str());
    assert_eq!(wire.preview_digest, state.request.preview_digest.to_hex());

    let request_id = state.request.id.clone();
    let mut app = yoctui_model::App::new(16, 4096);
    app.raw_mode
        .execution_states
        .insert(request_id.clone(), state);
    assert_eq!(
        yoctui_model::update(
            &mut app,
            yoctui_model::Action::RawMode(yoctui_model::RawModeAction::CancelExecution(
                request_id.clone()
            ),),
        ),
        Some(yoctui_model::Effect::CancelRaw(request_id))
    );
}
