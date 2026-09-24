//! Config inspection.
use super::*;

pub(crate) fn config_variable_loaded_action(
    requested: VariableIdentity,
    variable: VariableValue,
) -> Action {
    Action::VariableLoaded(VariableDetail {
        identity: VariableIdentity {
            name: requested.name,
            recipe: variable.recipe,
        },
        effective_value: variable.value,
        unexpanded_value: variable.unexpanded_value,
        provenance: variable.provenance,
        operations: variable.operations,
        active_overrides: variable.active_overrides,
    })
}

pub(crate) async fn load_config_variable(
    app: &mut App,
    backend: &mut dyn BitBakeBackend,
    identity: VariableIdentity,
) {
    match backend
        .get_variable(identity.name.clone(), identity.recipe.clone())
        .await
    {
        Ok(variable) => {
            let _ = update(app, config_variable_loaded_action(identity, variable));
        }
        Err(error) => {
            let _ = update(
                app,
                Action::VariableDetailFailed {
                    identity,
                    message: error.to_string(),
                },
            );
        }
    }
}

pub(crate) async fn inspect_selected_config_variable(
    app: &mut App,
    backend: &mut dyn BitBakeBackend,
) {
    if let Some(Effect::GetVariable(identity)) =
        compatibility_workspace_action(app, Action::BeginSelectedConfigDetail)
    {
        load_config_variable(app, backend, identity).await;
    }
}

pub(crate) fn config_copy_effect(app: &mut App, input: Input) -> Option<Effect> {
    config_workspace_action(false, input)
        .and_then(|action| compatibility_workspace_action(app, action))
}
