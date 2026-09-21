use super::*;

#[test]
fn raw_catalog_model_accepts_valid_bounded_typed_catalog() {
    let catalog = valid_catalog().normalize().unwrap();
    assert_eq!(catalog.categories[0].id.as_str(), "task-control");
    let RawExecutionPolicy::Executable { template } = &catalog.commands[0].execution else {
        panic!("fixture must be executable");
    };
    assert_eq!(
        template.display_template(&catalog.commands[0].parameters),
        Some("bitbake -c <task> <recipe>".into())
    );
}
