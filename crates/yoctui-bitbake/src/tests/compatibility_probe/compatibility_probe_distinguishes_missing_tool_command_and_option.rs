use super::*;

#[tokio::test]
async fn compatibility_probe_distinguishes_missing_tool_command_and_option() {
    let fixture =
        Fixture::new("#!/bin/sh\nif [ \"$1\" = bad ]; then exit 2; fi\necho 'modify finish'\n");
    let context = fixture.context();
    let runner = CapabilityProbeRunner::default();
    let missing_tool = runner
        .probe(
            &context,
            &CapabilityProbeSpec::Executable {
                tool: CapabilityToolId::Wic,
            },
        )
        .await;
    assert_eq!(missing_tool.status, CapabilityProbeStatus::Negative);
    let command = runner
        .probe(
            &context,
            &CapabilityProbeSpec::CommandHelp {
                tool: CapabilityToolId::Devtool,
                subcommand: Some("bad".into()),
            },
        )
        .await;
    assert_eq!(command.status, CapabilityProbeStatus::Negative);
    let option = runner
        .probe(
            &context,
            &CapabilityProbeSpec::CommandOption {
                tool: CapabilityToolId::Devtool,
                subcommand: None,
                option: "--force".into(),
            },
        )
        .await;
    assert_eq!(option.status, CapabilityProbeStatus::Negative);
}
