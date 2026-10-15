//! Config inspection.
use super::*;
mod live;
pub(crate) use live::ConfigurationIo;

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

pub(crate) fn config_copy_effect(app: &mut App, input: Input) -> Option<Effect> {
    config_workspace_action(false, input)
        .and_then(|action| compatibility_workspace_action(app, action))
}
