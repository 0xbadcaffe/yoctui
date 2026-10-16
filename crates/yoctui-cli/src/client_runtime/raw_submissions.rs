use std::collections::BTreeMap;
use yoctui_model::{Action, App, RawModeAction, RawRequestId};
use yoctui_protocol::daemon::{CommandOutcome, CommandResult};

pub(super) fn reconcile_raw_submission(
    app: &mut App,
    pending: &mut BTreeMap<u64, RawRequestId>,
    result: &CommandResult,
) {
    if let Some(request) = pending.remove(&result.request_id.0)
        && let CommandOutcome::Rejected { message, .. } = &result.outcome
    {
        yoctui_model::update(
            app,
            Action::RawMode(RawModeAction::SubmissionFailed {
                request,
                message: format!("Raw command was not started: {message}"),
            }),
        );
    }
}
