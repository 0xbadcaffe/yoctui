use super::*;

#[tokio::test]
async fn compatibility_probe_background_priority_reaches_the_tool_process() {
    let fixture =
        Fixture::new("#!/bin/sh\nsleep 0.1\nps -o ni= -p $$ > probe.nice\necho 'devtool 1.0'\n");
    let observation = CapabilityProbeRunner::default()
        .with_background_priority()
        .probe(
            &fixture.context(),
            &CapabilityProbeSpec::CommandVersion {
                tool: CapabilityToolId::Devtool,
            },
        )
        .await;
    assert_eq!(observation.status, CapabilityProbeStatus::Positive);
    assert_eq!(
        fs::read_to_string(fixture.root.join("probe.nice"))
            .unwrap()
            .trim(),
        "10"
    );
}
