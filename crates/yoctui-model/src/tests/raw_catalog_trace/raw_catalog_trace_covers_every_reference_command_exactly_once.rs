use super::*;

#[test]
fn raw_catalog_trace_covers_every_reference_command_exactly_once() {
    let (category_headings, reference_entries) = reference_entries();
    let catalog = RawCatalog::builtin();
    assert_eq!(reference_entries.len(), RAW_BUILTIN_COMMAND_COUNT);

    let mut seen_commands = BTreeSet::new();
    for reference in reference_entries {
        let reference_id = format!("wrynose-6-0.l{:04}", reference.line);
        let matches = catalog
            .commands
            .iter()
            .filter(|command| command.reference.id.as_str() == reference_id)
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 1, "reference line {}", reference.line);
        let command = matches[0];
        assert!(seen_commands.insert(command.id.clone()));
        assert_eq!(command.label, reference.command);
        assert_eq!(command.description, reference.description);
        assert_eq!(command.reference.heading, reference.heading);
        assert_eq!(command.reference.command, reference.command);
        assert_eq!(command.reference.description, reference.description);
        assert_eq!(
            catalog
                .category(&command.category)
                .unwrap()
                .reference_heading,
            reference.category_heading
        );

        match &command.execution {
            RawExecutionPolicy::Executable { template } => {
                assert!(direct_bitbake(reference.command), "{}", reference.command);
                assert_eq!(
                    template.display_template(&command.parameters).as_deref(),
                    Some(reference.command)
                );
            }
            RawExecutionPolicy::ReferenceOnly { kind, .. } => {
                assert!(!direct_bitbake(reference.command), "{}", reference.command);
                let expected = if reference.command.starts_with("bitbake ") {
                    RawReferenceKind::ShellPipeline
                } else {
                    RawReferenceKind::CompanionTool
                };
                assert_eq!(*kind, expected, "{}", reference.command);
                assert!(command.parameters.is_empty());
            }
        }
    }
    assert_eq!(seen_commands.len(), catalog.commands.len());

    let expected_headings = category_headings.into_iter().collect::<BTreeSet<_>>();
    let actual_headings = catalog
        .categories
        .iter()
        .filter(|category| category.kind != RawCategoryKind::Favorites)
        .map(|category| category.reference_heading.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_headings, expected_headings);
}
