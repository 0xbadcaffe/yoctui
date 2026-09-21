use super::*;

#[test]
fn devtool_status_uses_bounded_global_help_without_running_status() {
    let catalog = CapabilityCatalog::builtin();
    let status = catalog.entry(CapabilityId::DevtoolStatus).unwrap();

    assert!(status.probes.iter().any(|probe| matches!(
        probe,
        CapabilityProbeSpec::CommandHelpText {
            tool: CapabilityToolId::Devtool,
            needle,
        } if needle == "status"
    )));
    assert!(!status.probes.iter().any(|probe| matches!(
        probe,
        CapabilityProbeSpec::CommandHelp {
            tool: CapabilityToolId::Devtool,
            subcommand: Some(subcommand),
        } if subcommand == "status"
    )));
    assert_eq!(
        status.required_commands,
        vec![CommandRequirement {
            tool: CapabilityToolId::Devtool,
            subcommand: Some("status".into()),
            options: Vec::new(),
        }]
    );
}
