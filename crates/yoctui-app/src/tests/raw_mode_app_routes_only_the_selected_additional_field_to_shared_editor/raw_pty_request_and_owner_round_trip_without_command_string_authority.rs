use super::*;

#[test]
fn raw_pty_request_and_owner_round_trip_without_command_string_authority() {
    let mut state = raw_execution_state_fixture();
    state.request.interaction = yoctui_model::RawInteractionMode::InteractivePty;
    let wire = raw_execution_request_to_protocol(&state.request).unwrap();
    assert_eq!(
        wire.interaction,
        yoctui_protocol::daemon::RawInteractionData::InteractivePty
    );
    assert_eq!(
        raw_execution_request_from_protocol(&wire).unwrap(),
        state.request
    );
    assert_eq!(wire.command_id, "build.target");
    assert!(!wire.command_id.contains(' '));
}
