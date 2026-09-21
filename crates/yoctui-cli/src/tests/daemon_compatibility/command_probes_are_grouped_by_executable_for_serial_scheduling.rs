use super::*;

#[test]
fn command_probes_are_grouped_by_executable_for_serial_scheduling() {
    assert_eq!(
        probe_tool(&yoctui_model::CapabilityProbeSpec::CommandHelpText {
            tool: yoctui_model::CapabilityToolId::Devtool,
            needle: "status".into(),
        }),
        Some(yoctui_model::CapabilityToolId::Devtool)
    );
    assert_eq!(
        probe_tool(&yoctui_model::CapabilityProbeSpec::MetadataVariable {
            name: "MACHINE".into(),
        }),
        None
    );
}
