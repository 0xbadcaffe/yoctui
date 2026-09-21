use super::*;

#[test]
fn raw_catalog_model_rejects_unsafe_or_disagreeing_templates() {
    let mut unsafe_catalog = valid_catalog();
    let RawExecutionPolicy::Executable { template } = &mut unsafe_catalog.commands[0].execution
    else {
        unreachable!()
    };
    template.arguments[0] = RawArgument::Literal {
        value: ";rm".into(),
    };
    assert_eq!(
        unsafe_catalog.validate(),
        Err(RawCatalogError::UnsafeTemplate(id("task-control.run")))
    );

    let mut disagreement = valid_catalog();
    disagreement.commands[0].parameters.pop();
    assert_eq!(
        disagreement.validate(),
        Err(RawCatalogError::PlaceholderDisagreement(id(
            "task-control.run"
        )))
    );
}
