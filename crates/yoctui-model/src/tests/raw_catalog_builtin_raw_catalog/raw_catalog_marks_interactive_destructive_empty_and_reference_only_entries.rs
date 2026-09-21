use super::*;

#[test]
fn raw_catalog_marks_interactive_destructive_empty_and_reference_only_entries() {
    let devshell = command(607);
    let RawExecutionPolicy::Executable { template } = devshell.execution else {
        panic!("devshell must execute")
    };
    assert_eq!(template.interaction, RawInteractionMode::InteractivePty);

    let cleanall = command(593);
    let RawExecutionPolicy::Executable { template } = cleanall.execution else {
        panic!("cleanall must execute")
    };
    assert_eq!(template.safety, RawSafetyClass::Destructive);

    let empty_log = command(294);
    let RawExecutionPolicy::Executable { template } = empty_log.execution else {
        panic!("empty event-log argv must execute")
    };
    assert!(template.arguments.contains(&RawArgument::Empty));

    assert!(matches!(
        command(847).execution,
        RawExecutionPolicy::ReferenceOnly {
            kind: RawReferenceKind::ShellPipeline,
            ..
        }
    ));
    assert!(matches!(
        command(1932).execution,
        RawExecutionPolicy::ReferenceOnly {
            kind: RawReferenceKind::CompanionTool,
            ..
        }
    ));
}
