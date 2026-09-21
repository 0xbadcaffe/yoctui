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
