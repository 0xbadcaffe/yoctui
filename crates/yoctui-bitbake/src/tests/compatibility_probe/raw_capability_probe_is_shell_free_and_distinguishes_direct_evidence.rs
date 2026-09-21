use super::*;

#[tokio::test]
async fn raw_capability_probe_is_shell_free_and_distinguishes_direct_evidence() {
    let fixture = Fixture::new_tool(
        CapabilityToolId::BitBake,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" >> probe.argv\necho 'usage: bitbake --dry-run --runall'\n",
    );
    let context = fixture.context_for(CapabilityToolId::BitBake);
    let runner = CapabilityProbeRunner::default();
    let catalog = CapabilityCatalog::builtin();

    let positive = runner
        .probe(
            &context,
            &catalog
                .entry(CapabilityId::BitBakeRawDryRun)
                .unwrap()
                .probes[0],
        )
        .await;
    assert_eq!(positive.status, CapabilityProbeStatus::Positive);
    assert_eq!(positive.evidence.argv[1..], ["--help"]);

    let negative = runner
        .probe(
            &context,
            &catalog
                .entry(CapabilityId::BitBakeRawEventLog)
                .unwrap()
                .probes[0],
        )
        .await;
    assert_eq!(negative.status, CapabilityProbeStatus::Negative);
    assert_eq!(negative.evidence.argv[1..], ["--help"]);
    assert_eq!(
        fs::read_to_string(fixture.root.join("probe.argv")).unwrap(),
        "--help\n--help\n"
    );

    let oversized = Fixture::new_tool(
        CapabilityToolId::BitBake,
        "#!/bin/sh\nprintf '%080d\\n' 0\n",
    );
    let inconclusive = CapabilityProbeRunner::with_limits(Duration::from_secs(1), 16)
        .unwrap()
        .probe(
            &oversized.context_for(CapabilityToolId::BitBake),
            &catalog.entry(CapabilityId::BitBakeRawCli).unwrap().probes[0],
        )
        .await;
    assert_eq!(inconclusive.status, CapabilityProbeStatus::Inconclusive);
}
