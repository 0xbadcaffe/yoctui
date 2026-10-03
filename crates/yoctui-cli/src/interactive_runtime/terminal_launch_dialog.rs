//! Keep the existing chooser routes separate from the other editor dialogs.
use super::*;

impl InteractiveRuntime {
    pub(super) async fn route_terminal_launch_dialog(
        &mut self,
        input: Input,
    ) -> Result<KeyRouteOutcome> {
        let runtime = self;
        if native_metadata_scope::hold_pending_inspection_launch(
            &mut runtime.app,
            runtime.daemon_attached,
            runtime.recipe_inspection_operation.is_some()
                || runtime.platform_inspection_operation.is_some(),
            input,
        ) {
            return Ok(KeyRouteOutcome::Handled);
        }
        let effect = terminal_launch_dialog_action(input)
            .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        match effect {
            Some(effect @ Effect::Terminal(_)) => {
                let routed =
                    submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect);
                if routed == Some(false) {
                    runtime.app.cancel_pending_platform_menuconfig();
                } else if routed.is_none() {
                    if let Effect::Terminal(yoctui_model::TerminalEffect::Create {
                        kind: yoctui_model::TerminalCreationKind::GitUi,
                        program,
                        cwd,
                        arguments,
                        ..
                    }) = effect
                    {
                        if let Err(error) = runtime.guard.suspend() {
                            runtime.app.notification = Some(format!("Cannot open GitUI: {error}"));
                        } else {
                            let result = tokio::task::spawn_blocking(move || {
                                std::process::Command::new(program)
                                    .args(arguments)
                                    .current_dir(cwd)
                                    .status()
                            })
                            .await;
                            let restored = runtime.guard.resume();
                            runtime.app.notification = Some(match (result, restored) {
                                (_, Err(error)) => format!("Cannot restore terminal: {error}"),
                                (Ok(Ok(status)), Ok(())) if status.success() => {
                                    "GitUI closed; source status will refresh.".into()
                                }
                                (result, _) => format!("GitUI finished: {result:?}"),
                            });
                        }
                    } else {
                        runtime.app.cancel_pending_platform_menuconfig();
                        runtime.app.notification = Some("Embedded terminal unavailable: connect to the daemon or choose a detached terminal.".into());
                    }
                }
            }
            Some(Effect::LaunchDetachedTerminal(request)) => {
                begin_detached_terminal_launch(
                    &mut runtime.app,
                    &mut runtime.detached_terminal_operation,
                    request,
                );
            }
            _ => {}
        }
        Ok(KeyRouteOutcome::Handled)
    }
}
