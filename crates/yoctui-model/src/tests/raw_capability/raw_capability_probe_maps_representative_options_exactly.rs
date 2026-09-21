use super::*;

#[test]
fn raw_capability_probe_maps_representative_options_exactly() {
    let expected = [
        (145, CapabilityId::BitBakeRawUi),
        (201, CapabilityId::BitBakeRawDryRun),
        (325, CapabilityId::BitBakeRawServerToken),
        (1193, CapabilityId::BitBakeRawMulticonfig),
        (179, CapabilityId::BitBakeRawRunAll),
        (185, CapabilityId::BitBakeRawNoSetscene),
    ];
    for (line, capability) in expected {
        let command = command(line);
        let RawExecutionPolicy::Executable { template } = command.execution else {
            panic!("reference line {line} must be executable");
        };
        assert!(template.capabilities.capabilities().contains(&capability));
    }
}
