use std::path::PathBuf;
use yoctui_bitbake::{QaTaskCapabilityInput, QaTaskCapabilityInspector, QaTaskScopeInput};
use yoctui_model::RecipeIdentity;
use yoctui_protocol::daemon::{DaemonQaCapabilityInput, DaemonQaSnapshot};

pub fn inspect(input: DaemonQaCapabilityInput) -> Result<DaemonQaSnapshot, String> {
    let selected = RecipeIdentity {
        name: input.selected_recipe_name.clone(),
        file: PathBuf::from(input.selected_recipe_file),
    };
    let scope = QaTaskScopeInput {
        identity: selected.clone(),
        reported_tasks: input.recipe_names,
        family_tasks: Vec::new(),
        is_kernel: false,
        report_roots: Vec::new(),
    };
    let request = QaTaskCapabilityInput {
        release: None,
        build_directory: PathBuf::from(input.build_directory),
        selected,
        scopes: vec![scope],
    };
    let response = QaTaskCapabilityInspector::new(request)
        .inspect()
        .map_err(|error| error.to_string())?;
    let snapshot = response.snapshot();
    Ok(DaemonQaSnapshot {
        generation: input.generation,
        capability: if response.is_partial() {
            "partial".into()
        } else {
            "available".into()
        },
        task_bindings: snapshot
            .checks
            .iter()
            .map(|check| format!("{:?}", check))
            .collect(),
        reports: input.report_roots,
        limitations: snapshot.limitations.clone(),
    }
    .bounded())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_runtime_qa_adapter_rejects_unsafe_scope() {
        let input = DaemonQaCapabilityInput {
            generation: 1,
            build_directory: "relative".into(),
            source_directory: None,
            layer_directories: Vec::new(),
            recipe_names: Vec::new(),
            report_roots: Vec::new(),
            selected_recipe_name: "recipe".into(),
            selected_recipe_file: "/tmp/recipe.bb".into(),
        };
        assert!(inspect(input).is_err());
    }
}
