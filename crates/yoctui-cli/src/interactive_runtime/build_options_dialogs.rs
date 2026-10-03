//! Existing build options and target dialogs, delegated after devtool/config routes.
use super::*;

impl InteractiveRuntime {
    pub(super) async fn route_build_options_dialogs(
        &mut self,
        input: Input,
    ) -> Result<Option<KeyRouteOutcome>> {
        let runtime = self;
        if matches!(runtime.app.active_dialog(), Some(Dialog::BuildOptions)) {
            let effect = match input {
                Input::Char('b') => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::BeginBuildTargetTask(None),
                ),
                Input::Char('c') => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::BeginBuildTargetTask(Some("clean".into())),
                ),
                Input::Char('m') => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::BeginBuildTargetTask(Some("menuconfig".into())),
                ),
                Input::Char('e') => {
                    compatibility_workspace_action(&mut runtime.app, Action::BeginBuildTargetEdit)
                }
                Input::Char('i') => {
                    let images = runtime
                        .app
                        .workspace
                        .recipes
                        .iter()
                        .map(|recipe| recipe.name.as_str())
                        .filter(|name| name.contains("image"))
                        .map(str::to_owned)
                        .collect();
                    compatibility_workspace_action(
                        &mut runtime.app,
                        Action::OpenImageBuildPicker(images),
                    )
                }
                Input::Esc => {
                    compatibility_workspace_action(&mut runtime.app, Action::CloseBuildOptions)
                }
                _ => None,
            };
            if let Some(Effect::Start(request)) = effect {
                begin_runtime_build(
                    &mut runtime.daemon_runtime,
                    &mut runtime.backend,
                    &mut runtime.app,
                    &mut runtime.build_jobs,
                    request,
                )
                .await;
            }
        } else if let Some(Dialog::BuildTarget { editor, .. }) =
            runtime.app.active_dialog().cloned()
        {
            let action = match input {
                Input::Enter => Some(Action::ConfirmBuildTarget),
                Input::Char('q') | Input::Esc if !editor.editing => {
                    Some(Action::CancelBuildTargetEdit)
                }
                input => popup_editor_action(editor.editing, input),
            };
            let effect =
                action.and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            match effect {
                Some(Effect::Start(request)) => {
                    begin_runtime_build(
                        &mut runtime.daemon_runtime,
                        &mut runtime.backend,
                        &mut runtime.app,
                        &mut runtime.build_jobs,
                        request,
                    )
                    .await;
                }
                Some(Effect::CopyToClipboard(content)) => {
                    copy_to_clipboard(&mut runtime.app, content).await;
                }
                _ => {}
            }
        } else {
            return Ok(None);
        }
        Ok(Some(KeyRouteOutcome::Handled))
    }
}
