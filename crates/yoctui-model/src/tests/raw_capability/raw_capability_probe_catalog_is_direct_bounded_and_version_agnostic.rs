use super::*;

#[test]
fn raw_capability_probe_catalog_is_direct_bounded_and_version_agnostic() {
    let catalog = CapabilityCatalog::builtin();
    catalog.validate().unwrap();
    for id in CapabilityId::RAW_CLI {
        let entry = catalog.entry(id).unwrap();
        assert_eq!(entry.required_tools, vec![CapabilityToolId::BitBake]);
        assert_eq!(entry.probes.len(), 1);
        assert!(entry.fallback.is_none());
        let direct = match (&entry.probes[0], id) {
            (
                CapabilityProbeSpec::CommandHelp {
                    tool: CapabilityToolId::BitBake,
                    subcommand: None,
                },
                CapabilityId::BitBakeRawCli,
            ) => true,
            (
                CapabilityProbeSpec::CommandOption {
                    tool: CapabilityToolId::BitBake,
                    subcommand: None,
                    ..
                },
                id,
            ) => id != CapabilityId::BitBakeRawMulticonfig,
            (
                CapabilityProbeSpec::CommandHelpText {
                    tool: CapabilityToolId::BitBake,
                    needle,
                },
                CapabilityId::BitBakeRawMulticonfig,
            ) => needle == "mc:",
            _ => false,
        };
        assert!(direct, "{}", id.as_str());
    }

    for command in RawCatalog::builtin().commands {
        if let RawExecutionPolicy::Executable { template } = command.execution {
            assert!(
                template
                    .capabilities
                    .capabilities()
                    .contains(&CapabilityId::BitBakeRawCli)
            );
        }
    }

    let absent = command(201).availability(None);
    assert_eq!(absent.state, RawAvailabilityState::Unknown);
    assert!(absent.issues.iter().any(|issue| {
        issue.capability == Some(CapabilityId::BitBakeRawCli)
            && issue
                .reason
                .contains("No current environment capability snapshot")
    }));
    assert_eq!(
        command(847).availability(None).state,
        RawAvailabilityState::Unsupported
    );
}
