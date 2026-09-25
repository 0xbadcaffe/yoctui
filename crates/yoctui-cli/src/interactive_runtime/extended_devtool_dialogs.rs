use super::*;

impl InteractiveRuntime {
    pub(super) async fn route_extended_devtool_dialogs(
        &mut self,
        input: Input,
    ) -> Result<Option<KeyRouteOutcome>> {
        if matches!(
            self.app.active_dialog(),
            Some(Dialog::DevtoolUndeployConfirmation(_))
        ) {
            let effect = devtool_undeploy_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut self.app, action));
            if let Some(Effect::DevtoolUndeploy(plan)) = effect {
                if submit_daemon_effect(
                    &mut self.daemon_runtime,
                    &mut self.app,
                    &Effect::DevtoolUndeploy(plan.clone()),
                )
                .is_some()
                {
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                if begin_devtool_job(
                    &mut self.app,
                    &mut self.devtool_jobs,
                    &mut self.devtool_runner,
                    &self.session_build_dir,
                    self.cancellation_timeout,
                    None,
                    plan.operation(),
                )
                .await
                {
                    self.pending_devtool_undeploy = Some(plan.identity);
                }
            }
            return Ok(Some(KeyRouteOutcome::Handled));
        }
        if matches!(self.app.active_dialog(), Some(Dialog::DevtoolUndeploy(_))) {
            let _ = devtool_undeploy_dialog_action(input)
                .and_then(|action| compatibility_workspace_action(&mut self.app, action));
            return Ok(Some(KeyRouteOutcome::Handled));
        }
        if matches!(
            self.app.active_dialog(),
            Some(Dialog::DevtoolUpgradeConfirmation(_))
        ) {
            let effect = devtool_upgrade_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut self.app, action));
            if let Some(Effect::DevtoolUpgrade(plan)) = effect {
                if submit_daemon_effect(
                    &mut self.daemon_runtime,
                    &mut self.app,
                    &Effect::DevtoolUpgrade(plan.clone()),
                )
                .is_some()
                {
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                if begin_devtool_job(
                    &mut self.app,
                    &mut self.devtool_jobs,
                    &mut self.devtool_runner,
                    &self.session_build_dir,
                    self.cancellation_timeout,
                    None,
                    plan.operation(),
                )
                .await
                {
                    self.pending_devtool_upgrade = Some(plan.identity);
                }
            }
            return Ok(Some(KeyRouteOutcome::Handled));
        }
        Ok(None)
    }
}
