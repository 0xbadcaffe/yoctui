use super::*;

fn selector_command(kind: RawParameterKind) -> (RawCommand, RawParameterId) {
    let command = RawCatalog::builtin()
        .commands
        .into_iter()
        .find(|command| {
            matches!(command.execution, RawExecutionPolicy::Executable { .. })
                && command
                    .parameters
                    .iter()
                    .any(|parameter| parameter.kind == kind)
        })
        .unwrap();
    let parameter = command
        .parameters
        .iter()
        .find(|parameter| parameter.kind == kind)
        .unwrap()
        .id
        .clone();
    (command, parameter)
}

mod raw_selector_distinguishes_absent_and_authoritative_empty_inventories;

mod raw_selector_retains_exact_recipe_and_typed_inventory_identities;

mod raw_selector_correlates_tasks_to_the_exact_recipe_and_replaces_results;

mod raw_selector_manual_target_and_task_entry_uses_parameter_validation;
