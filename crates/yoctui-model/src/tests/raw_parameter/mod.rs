use super::*;

fn parameter(kind: RawParameterKind, presence: RawParameterPresence) -> RawParameter {
    RawParameter {
        id: RawParameterId::new("value").unwrap(),
        label: "Value".into(),
        placeholder: "<value>".into(),
        kind,
        presence,
    }
}

fn parsed(kind: RawParameterKind, input: &str) -> RawParameterValue {
    parameter(kind, RawParameterPresence::Required)
        .parse_value(input)
        .unwrap()
        .unwrap()
}

mod raw_parameter_accepts_every_typed_kind_as_one_argument;

mod raw_parameter_required_and_optional_empty_inputs_remain_distinct;

mod raw_parameter_enforces_identifier_and_unicode_byte_boundaries;

mod raw_parameter_rejects_shell_syntax_traversal_and_invalid_numbers;

mod raw_parameter_rejects_typed_kind_definition_disagreement;
