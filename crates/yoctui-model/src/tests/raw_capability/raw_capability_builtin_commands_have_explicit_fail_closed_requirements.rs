use super::*;

#[test]
fn raw_capability_builtin_commands_have_explicit_fail_closed_requirements() {
    let catalog = RawCatalog::builtin();
    for command in &catalog.commands {
        match &command.execution {
            RawExecutionPolicy::Executable { template } => {
                assert!(!template.capabilities.capabilities().is_empty());
                let availability = command.availability(None);
                assert_eq!(availability.state, RawAvailabilityState::Unknown);
                assert!(!availability.issues.is_empty());
            }
            RawExecutionPolicy::ReferenceOnly { reason, .. } => {
                let availability = command.availability(None);
                assert_eq!(availability.state, RawAvailabilityState::Unsupported);
                assert_eq!(availability.issues[0].reason, *reason);
                assert!(availability.implementations.is_empty());
            }
        }
    }
}
