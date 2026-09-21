use super::*;

fn command(line: usize) -> RawCommand {
    RawCatalog::builtin()
        .commands
        .into_iter()
        .find(|command| command.reference.id.as_str() == format!("wrynose-6-0.l{line:04}"))
        .unwrap()
}

mod raw_catalog_builtin_is_complete_bounded_and_valid;

mod raw_catalog_preserves_exact_help_and_structured_templates;

mod raw_catalog_marks_interactive_destructive_empty_and_reference_only_entries;

mod raw_catalog_does_not_present_conceptual_or_companion_sections_as_executable;
