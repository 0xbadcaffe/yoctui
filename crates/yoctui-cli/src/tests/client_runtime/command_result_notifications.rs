use crate::client_runtime::replica::command_result_notification;
use yoctui_protocol::daemon::{CommandOutcome, CommandResult, ProtocolErrorCode, RequestId};

#[test]
fn accepted_requests_defer_to_typed_daemon_activity() {
    let result = CommandResult {
        request_id: RequestId(7),
        outcome: CommandOutcome::Accepted,
    };

    assert_eq!(command_result_notification(result), None);
}

#[test]
fn rejected_requests_remain_visible() {
    let result = CommandResult {
        request_id: RequestId(8),
        outcome: CommandOutcome::Rejected {
            code: ProtocolErrorCode::Conflict,
            message: "another request is active".into(),
            current_generation: 4,
        },
    };

    assert_eq!(
        command_result_notification(result),
        Some("Daemon request 8 was rejected: another request is active".into())
    );
}
