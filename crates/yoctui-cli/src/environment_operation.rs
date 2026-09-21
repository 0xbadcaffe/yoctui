//! Environment setup and metadata discovery never hold the terminal event loop.
use super::*;
type Connected = (Box<dyn BitBakeBackend>, yoctui_model::Workspace);
pub(crate) struct EnvironmentOperation {
    task: tokio::task::JoinHandle<Result<Connected>>,
    generation: u64,
}
impl Drop for EnvironmentOperation {
    fn drop(&mut self) {
        self.task.abort();
    }
}
pub(crate) fn start(
    slot: &mut Option<EnvironmentOperation>,
    profile: yoctui_model::BuildEnvironmentProfile,
    generation: u64,
    kind: Backend,
    cancellation_timeout: Duration,
) {
    let task = tokio::spawn(async move {
        let response = BuildEnvironmentAdapter::default()
            .initialize(profile)
            .await?;
        let mut backend = select_backend_with_environment(
            kind,
            response.profile.build_dir,
            Some(cancellation_timeout),
            Some(response.environment),
        )
        .await?;
        let workspace = backend.inspect_workspace().await?;
        Ok((backend, workspace))
    });
    *slot = Some(EnvironmentOperation { task, generation });
}
pub(crate) async fn poll(
    app: &mut App,
    backend: &mut Box<dyn BitBakeBackend>,
    slot: &mut Option<EnvironmentOperation>,
) {
    if !slot.as_ref().is_some_and(|op| op.task.is_finished()) {
        return;
    }
    let mut op = slot.take().expect("finished environment operation");
    let result = (&mut op.task).await;
    if !matches!(app.build_environment, yoctui_model::BuildEnvironmentState::Verifying { generation, .. } if generation == op.generation)
    {
        return;
    }
    match result {
        Ok(Ok((connected, workspace))) => {
            compatibility_workspace_action(
                app,
                Action::BuildEnvironmentVerified {
                    generation: op.generation,
                },
            );
            compatibility_workspace_action(app, Action::WorkspaceLoaded(workspace));
            let mut previous = std::mem::replace(backend, connected);
            tokio::spawn(async move {
                let _ = tokio::time::timeout(Duration::from_secs(5), previous.shutdown()).await;
            });
        }
        result => {
            let message = match result {
                Ok(Err(e)) => e.to_string(),
                Err(e) => e.to_string(),
                _ => unreachable!(),
            };
            compatibility_workspace_action(
                app,
                Action::BuildEnvironmentVerificationFailed {
                    generation: op.generation,
                    message,
                },
            );
        }
    }
}

#[cfg(test)]
#[path = "tests/environment_operation/mod.rs"]
mod tests;
