//! Clone ownership is independent of terminal input and rendering.
use std::time::Duration;
use tokio::task::JoinHandle;
use yoctui_app::compatibility_workspace_action;
use yoctui_bitbake::BuildEnvironmentAdapter;
use yoctui_model::{
    Action, App, BackgroundActivity, BuildEnvironmentClonePlan, BuildEnvironmentProfile,
    BuildEnvironmentState,
};

pub(crate) struct CloneOperation {
    task: JoinHandle<Result<(), String>>,
    plan: BuildEnvironmentClonePlan,
    original: BuildEnvironmentState,
}
impl Drop for CloneOperation {
    fn drop(&mut self) {
        self.task.abort();
    }
}
pub(crate) fn start(
    app: &mut App,
    slot: &mut Option<CloneOperation>,
    plan: BuildEnvironmentClonePlan,
) {
    if slot.is_some() {
        return;
    }
    let request = plan.request.clone();
    let task = tokio::spawn(async move {
        BuildEnvironmentAdapter::new(Duration::from_secs(1800))
            .clone_poky(request.clone())
            .await
            .map_err(|e| e.to_string())?;
        if !request.destination.join("oe-init-build-env").is_file() {
            return Err(
                "clone has no oe-init-build-env script; select a Yocto source repository".into(),
            );
        }
        Ok(())
    });
    *slot = Some(CloneOperation {
        task,
        plan,
        original: app.build_environment.clone(),
    });
    activity(app, true);
}
fn activity(app: &mut App, active: bool) {
    compatibility_workspace_action(
        app,
        Action::SetBackgroundActivity {
            activity: BackgroundActivity::Cloning,
            active,
        },
    );
}
pub(crate) fn cancel(app: &mut App, slot: &mut Option<CloneOperation>) {
    if slot.take().is_some() {
        activity(app, false);
        app.notification = Some(
            "Clone cancelled. Partial files are retained; choose a new empty destination to retry."
                .into(),
        );
    }
}
pub(crate) async fn poll(app: &mut App, slot: &mut Option<CloneOperation>) {
    if !slot.as_ref().is_some_and(|op| op.task.is_finished()) {
        return;
    }
    let mut operation = slot.take().expect("finished clone");
    let outcome = (&mut operation.task).await;
    activity(app, false);
    match outcome {
        Ok(Ok(())) if app.build_environment == operation.original => {
            let destination = &operation.plan.request.destination;
            compatibility_workspace_action(app, Action::ConfigureBuildEnvironment(BuildEnvironmentProfile {
                source_dir: destination.clone(), build_dir: operation.plan.build_dir.clone(), init_script: destination.join("oe-init-build-env"),
            }));
            app.notification = Some("Poky cloned. Press V to initialize and verify BitBake.".into());
        }
        Ok(Ok(())) => app.notification = Some("Clone completed. Environment changed during cloning; select the cloned source in Build Environment.".into()),
        Ok(Err(error)) => app.notification = Some(format!("Poky clone failed: {error}")),
        Err(error) => app.notification = Some(format!("Clone worker failed: {error}")),
    }
}

#[cfg(test)]
#[path = "tests/clone_operation/mod.rs"]
mod tests;
