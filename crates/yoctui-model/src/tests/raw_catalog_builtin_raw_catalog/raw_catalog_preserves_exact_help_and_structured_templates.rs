use super::*;

#[test]
fn raw_catalog_preserves_exact_help_and_structured_templates() {
    let task = command(167);
    assert_eq!(task.description, "Execute one named task for a recipe.");
    let RawExecutionPolicy::Executable { template } = task.execution else {
        panic!("task command must execute")
    };
    assert_eq!(
        template.display_template(&task.parameters).as_deref(),
        Some("bitbake -c <task> <recipe>")
    );

    let joined = command(145);
    let RawExecutionPolicy::Executable { template } = joined.execution else {
        panic!("UI command must execute")
    };
    assert!(matches!(
        template.arguments[0],
        RawArgument::JoinedParameter { .. }
    ));

    let composed = command(87);
    let RawExecutionPolicy::Executable { template } = composed.execution else {
        panic!("task syntax must execute")
    };
    assert!(matches!(
        template.arguments[0],
        RawArgument::Composed { .. }
    ));
}
