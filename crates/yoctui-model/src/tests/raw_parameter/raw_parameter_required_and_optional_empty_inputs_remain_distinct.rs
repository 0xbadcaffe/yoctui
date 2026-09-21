use super::*;

#[test]
fn raw_parameter_required_and_optional_empty_inputs_remain_distinct() {
    let required = parameter(RawParameterKind::Target, RawParameterPresence::Required);
    assert_eq!(
        required.parse_value(""),
        Err(RawParameterError::Required {
            parameter: required.id.clone(),
        })
    );

    let optional = parameter(RawParameterKind::Target, RawParameterPresence::Optional);
    assert_eq!(optional.parse_value(""), Ok(None));
    assert_eq!(
        optional.parse_value("busybox").unwrap(),
        Some(RawParameterValue::Target("busybox".into()))
    );
    assert!(optional.parse_value(" ").is_err());
}
