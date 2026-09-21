use super::*;

#[test]
fn raw_parameter_rejects_typed_kind_definition_disagreement() {
    let recipe = parameter(RawParameterKind::Recipe, RawParameterPresence::Required);
    assert_eq!(
        recipe.validate_value(&RawParameterValue::Target("busybox".into())),
        Err(RawParameterError::KindMismatch {
            parameter: recipe.id.clone(),
            expected: RawParameterKind::Recipe,
            actual: RawParameterKind::Target,
        })
    );
}
