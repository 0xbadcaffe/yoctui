use super::*;

#[test]
fn daemon_qa_input_is_bounded() {
    let input = DaemonQaCapabilityInput {
        generation: 1,
        build_directory: "/build".into(),
        source_directory: None,
        layer_directories: (0..MAX_QA_RECORDS + 1).map(|i| i.to_string()).collect(),
        recipe_names: Vec::new(),
        report_roots: Vec::new(),
        selected_recipe_name: "recipe".into(),
        selected_recipe_file: "/build/recipe.bb".into(),
    }
    .bounded();
    assert_eq!(input.layer_directories.len(), MAX_QA_RECORDS);
}
