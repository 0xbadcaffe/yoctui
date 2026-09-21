use super::*;
fn fixture(app: &App, task: JoinHandle<Result<(), String>>) -> CloneOperation {
    CloneOperation {
        task,
        original: app.build_environment.clone(),
        plan: BuildEnvironmentClonePlan {
            request: yoctui_model::BuildEnvironmentCloneRequest {
                repository: "https://example.invalid/poky".into(),
                destination: "/tmp/cloned-poky".into(),
                revision: None,
            },
            build_dir: "/tmp/cloned-poky/build".into(),
        },
    }
}
mod clone_completion_and_failure_clear_pending_state;
mod clone_does_not_overwrite_changed_environment;
mod clone_pending_poll_does_not_wait_and_cancel_clears_activity;
