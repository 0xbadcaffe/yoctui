use super::*;

pub(super) async fn route_dependency_workspace(
    runtime: &mut InteractiveRuntime,
    input: Input,
) -> bool {
    if runtime.app.screen != Screen::Dependencies {
        return false;
    }
    let Some(action) = dependency_workspace_action(runtime.app.dependency_graph_searching, input)
    else {
        return false;
    };
    match compatibility_workspace_action(&mut runtime.app, action) {
        Some(Effect::GetDependencies(recipe)) => {
            load_dependency_graph(&mut runtime.app, runtime.backend.as_mut(), recipe).await;
        }
        Some(Effect::OpenInEditor(path)) => {
            open_in_editor(
                &runtime.guard,
                &mut runtime.app,
                path,
                runtime.editor_command.as_deref(),
            )
            .await;
        }
        _ => {}
    }
    true
}
