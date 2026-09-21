use super::*;

fn parse(input: &str) -> Result<Vec<String>, RawArgvError> {
    RawAdditionalArguments::parse(input).map(RawAdditionalArguments::into_vec)
}

mod raw_argv_tokenizes_quotes_escapes_empty_elements_and_unicode_as_native_arguments;

mod raw_security_argv_rejects_every_documented_operator_even_when_quoted_or_assembled;

mod raw_argv_reports_controls_unterminated_grammar_and_empty_option_names;

mod raw_argv_enforces_input_element_count_and_aggregate_byte_boundaries;

mod raw_argv_editor_invalidates_stale_validation_and_replaces_failures;
