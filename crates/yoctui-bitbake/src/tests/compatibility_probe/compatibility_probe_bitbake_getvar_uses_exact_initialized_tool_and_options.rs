use super::*;

#[tokio::test]
async fn compatibility_probe_bitbake_getvar_uses_exact_initialized_tool_and_options() {
    let fixture = Fixture::new_tool(
        CapabilityToolId::BitBakeGetVar,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" >> probe.argv\necho 'usage: bitbake-getvar [--value] [-r RECIPE] variable'\necho '  -r, --recipe RECIPE'\n",
    );
    let context = fixture.context_for(CapabilityToolId::BitBakeGetVar);
    let runner = CapabilityProbeRunner::default();
    for probe in [
        CapabilityProbeSpec::Executable {
            tool: CapabilityToolId::BitBakeGetVar,
        },
        CapabilityProbeSpec::CommandHelp {
            tool: CapabilityToolId::BitBakeGetVar,
            subcommand: None,
        },
        CapabilityProbeSpec::CommandOption {
            tool: CapabilityToolId::BitBakeGetVar,
            subcommand: None,
            option: "--value".into(),
        },
        CapabilityProbeSpec::CommandOption {
            tool: CapabilityToolId::BitBakeGetVar,
            subcommand: None,
            option: "--recipe".into(),
        },
    ] {
        assert_eq!(
            runner.probe(&context, &probe).await.status,
            CapabilityProbeStatus::Positive
        );
    }
    assert_eq!(
        fs::read_to_string(fixture.root.join("probe.argv")).unwrap(),
        "--help\n--help\n--help\n"
    );

    let missing_context = Fixture::new("#!/bin/sh\necho ok\n").context();
    assert_eq!(
        runner
            .probe(
                &missing_context,
                &CapabilityProbeSpec::Executable {
                    tool: CapabilityToolId::BitBakeGetVar
                }
            )
            .await
            .status,
        CapabilityProbeStatus::Negative
    );
}
