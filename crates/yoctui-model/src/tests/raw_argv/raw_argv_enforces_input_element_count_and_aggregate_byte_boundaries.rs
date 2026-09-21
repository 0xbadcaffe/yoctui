use super::*;

#[test]
fn raw_argv_enforces_input_element_count_and_aggregate_byte_boundaries() {
    assert!(parse(&" ".repeat(MAX_RAW_ADDITIONAL_INPUT_BYTES)).is_ok());
    assert_eq!(
        parse(&" ".repeat(MAX_RAW_ADDITIONAL_INPUT_BYTES + 1)),
        Err(RawArgvError::InputTooLong {
            bytes: MAX_RAW_ADDITIONAL_INPUT_BYTES + 1,
            maximum: MAX_RAW_ADDITIONAL_INPUT_BYTES,
        })
    );

    assert!(parse(&"a".repeat(MAX_RAW_ADDITIONAL_ARGUMENT_BYTES)).is_ok());
    assert!(matches!(
        parse(&"a".repeat(MAX_RAW_ADDITIONAL_ARGUMENT_BYTES + 1)),
        Err(RawArgvError::ArgumentTooLong { .. })
    ));
    let unicode = "é".repeat(MAX_RAW_ADDITIONAL_ARGUMENT_BYTES / 2);
    assert_eq!(unicode.len(), MAX_RAW_ADDITIONAL_ARGUMENT_BYTES);
    assert!(parse(&unicode).is_ok());
    assert!(matches!(
        parse(&(unicode + "é")),
        Err(RawArgvError::ArgumentTooLong { .. })
    ));

    let maximum_count = std::iter::repeat_n("x", MAX_RAW_ADDITIONAL_ARGUMENTS)
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        parse(&maximum_count).unwrap().len(),
        MAX_RAW_ADDITIONAL_ARGUMENTS
    );
    assert!(matches!(
        parse(&(maximum_count + " x")),
        Err(RawArgvError::TooManyArguments { .. })
    ));

    let aggregate = std::iter::repeat_n(
        "a".repeat(MAX_RAW_ADDITIONAL_ARGUMENT_BYTES),
        MAX_RAW_ADDITIONAL_AGGREGATE_BYTES / MAX_RAW_ADDITIONAL_ARGUMENT_BYTES,
    )
    .collect::<Vec<_>>()
    .join(" ");
    assert!(parse(&aggregate).is_ok());
    assert!(matches!(
        parse(&(aggregate + " x")),
        Err(RawArgvError::AggregateTooLong { .. })
    ));
}
