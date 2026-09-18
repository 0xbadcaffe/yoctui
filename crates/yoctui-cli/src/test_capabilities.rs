//! Test capabilities.
use super::*;

pub(crate) fn ptest_capability(app: &App) -> yoctui_model::PtestCapability {
    let suites = app.workspace.variables.get("TEST_SUITES");
    let features = app
        .workspace
        .variables
        .get("EXTRA_IMAGE_FEATURES")
        .or_else(|| app.workspace.variables.get("IMAGE_FEATURES"));
    match (suites, features) {
        (Some(suites), Some(features))
            if suites.split_whitespace().any(|value| value == "ptest")
                && features
                    .split_whitespace()
                    .any(|value| value == "ptest-pkgs") =>
        {
            yoctui_model::PtestCapability::Configured
        }
        (Some(_), Some(_)) => yoctui_model::PtestCapability::Unavailable(
            "active TEST_SUITES/IMAGE_FEATURES do not confirm ptest".into(),
        ),
        _ => yoctui_model::PtestCapability::Unavailable(
            "active TEST_SUITES and image features are unavailable".into(),
        ),
    }
}

pub(crate) fn testing_screen_action(app: &App, input: Input) -> Option<Action> {
    match app.test_view {
        TestWorkspaceView::Launches => testing_workspace_action(input),
        TestWorkspaceView::Results => {
            test_results_workspace_action(app.test_result_searching, app.test_result_drilled, input)
        }
        TestWorkspaceView::Comparison => test_comparison_workspace_action(input),
    }
}
