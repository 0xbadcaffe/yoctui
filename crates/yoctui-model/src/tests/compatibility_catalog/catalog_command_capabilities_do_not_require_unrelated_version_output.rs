use super::*;

#[test]
fn catalog_command_capabilities_do_not_require_unrelated_version_output() {
    let catalog = CapabilityCatalog::builtin();
    let devtool = catalog.entry(CapabilityId::DevtoolUpgrade).unwrap();

    assert!(devtool.probes.iter().any(|probe| matches!(
        probe,
        CapabilityProbeSpec::CommandHelp {
            tool: CapabilityToolId::Devtool,
            subcommand: Some(subcommand),
        } if subcommand == "upgrade"
    )));
    assert!(
        !devtool
            .probes
            .iter()
            .any(|probe| matches!(probe, CapabilityProbeSpec::CommandVersion { .. }))
    );
}
