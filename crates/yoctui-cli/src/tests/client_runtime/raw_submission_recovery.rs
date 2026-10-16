use super::*;
use std::collections::BTreeMap;
use yoctui_protocol::daemon::{CommandOutcome, CommandResult, ProtocolErrorCode, RequestId};

#[test]
fn raw_rejected_submission_is_correlated_and_leaves_connecting_without_fake_result() {
    let mut app = App::new(8, 1024);
    let request = yoctui_model::RawRequestId::new("raw-request:pending").unwrap();
    app.raw_mode.view = yoctui_model::RawModeView::Execution;
    app.raw_mode.output.request = Some(request.clone());
    let mut pending = BTreeMap::from([(9, request)]);
    let mut result = CommandResult {
        request_id: RequestId(8),
        outcome: CommandOutcome::Rejected {
            code: ProtocolErrorCode::Conflict,
            message: "daemon generation changed".into(),
            current_generation: 5,
        },
    };
    super::super::raw_submissions::reconcile_raw_submission(&mut app, &mut pending, &result);
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Execution);
    assert_eq!(pending.len(), 1);
    result.request_id = RequestId(9);
    super::super::raw_submissions::reconcile_raw_submission(&mut app, &mut pending, &result);
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Browser);
    assert!(pending.is_empty());
    assert!(app.raw_mode.output.request.is_none());
    assert!(app.raw_mode.execution_states.is_empty());
    assert!(
        app.raw_mode
            .notification
            .as_deref()
            .unwrap()
            .contains("was not started")
    );
}

#[test]
fn raw_accepted_submission_waits_for_authoritative_execution() {
    let mut app = App::new(8, 1024);
    let request = yoctui_model::RawRequestId::new("raw-request:accepted").unwrap();
    app.raw_mode.view = yoctui_model::RawModeView::Execution;
    app.raw_mode.output.request = Some(request.clone());
    let mut pending = BTreeMap::from([(3, request.clone())]);
    super::super::raw_submissions::reconcile_raw_submission(
        &mut app,
        &mut pending,
        &CommandResult {
            request_id: RequestId(3),
            outcome: CommandOutcome::Accepted,
        },
    );
    assert!(pending.is_empty());
    assert_eq!(app.raw_mode.view, yoctui_model::RawModeView::Execution);
    assert_eq!(app.raw_mode.output.request.as_ref(), Some(&request));
    assert!(app.raw_mode.notification.is_none());
}
