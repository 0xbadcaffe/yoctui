use super::*;

#[test]
fn raw_catalog_trace_counts_classifications_and_unique_references() {
    let catalog = RawCatalog::builtin();
    let executable = catalog
        .commands
        .iter()
        .filter(|command| matches!(command.execution, RawExecutionPolicy::Executable { .. }))
        .count();
    let references = catalog
        .commands
        .iter()
        .map(|command| command.reference.id.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(catalog.categories.len(), RAW_BUILTIN_CATEGORY_COUNT);
    assert_eq!(executable, RAW_BUILTIN_EXECUTABLE_COUNT);
    assert_eq!(references.len(), RAW_BUILTIN_COMMAND_COUNT);
    assert_eq!(RAW_REFERENCE_SHA256.len(), 64);
}
