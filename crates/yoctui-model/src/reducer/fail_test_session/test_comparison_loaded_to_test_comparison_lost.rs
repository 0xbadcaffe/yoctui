use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::TestComparisonLoaded {
            request,
            comparison,
            limitations,
        } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            let baseline = app
                .test_results
                .records()
                .iter()
                .find(|record| record.identity == request.baseline);
            let candidate = app
                .test_results
                .records()
                .iter()
                .find(|record| record.identity == request.candidate);
            let Some(expected) = baseline.zip(candidate).and_then(|(baseline, candidate)| {
                TestComparison::between(baseline, candidate).ok()
            }) else {
                note_stale_test_event(app);
                return None;
            };
            if comparison != expected {
                note_stale_test_event(app);
                app.notification =
                    Some("Testing rejected an inconsistent comparison result.".into());
                return None;
            }
            let limitations = normalize_limitations(limitations);
            app.test_comparison = if limitations.is_empty() {
                TestComparisonState::Available {
                    request,
                    comparison,
                }
            } else {
                TestComparisonState::Partial {
                    request,
                    comparison,
                    limitations,
                }
            };
            set_test_comparison_selection(app);
        }
        Action::TestComparisonFailed { request, message } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_comparison = TestComparisonState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("Test comparison failed: {message}"));
        }
        Action::TestComparisonCancelled { request } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_comparison = TestComparisonState::Cancelled { request };
        }
        Action::TestComparisonTimedOut { request } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_comparison = TestComparisonState::TimedOut { request };
        }
        Action::TestComparisonLost { request, message } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_comparison = TestComparisonState::Lost { request, message };
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
