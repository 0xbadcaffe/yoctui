use super::*;

#[test]
fn raw_catalog_builtin_is_complete_bounded_and_valid() {
    let catalog = RawCatalog::builtin();
    catalog.validate().unwrap();
    assert_eq!(catalog.categories.len(), RAW_BUILTIN_CATEGORY_COUNT);
    assert_eq!(catalog.commands.len(), RAW_BUILTIN_COMMAND_COUNT);
    assert_eq!(
        catalog
            .commands
            .iter()
            .filter(|command| matches!(command.execution, RawExecutionPolicy::Executable { .. }))
            .count(),
        RAW_BUILTIN_EXECUTABLE_COUNT
    );
}
