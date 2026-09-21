use super::*;

#[tokio::test]
async fn compatibility_probe_reports_timeout_oversize_and_stale_executable_as_inconclusive() {
    let timeout_fixture = Fixture::new("#!/bin/sh\nsleep 30\n");
    let runner = CapabilityProbeRunner::with_limits(Duration::from_millis(30), 128).unwrap();
    let timed_out = runner
        .probe(
            &timeout_fixture.context(),
            &CapabilityProbeSpec::CommandVersion {
                tool: CapabilityToolId::Devtool,
            },
        )
        .await;
    assert_eq!(timed_out.status, CapabilityProbeStatus::Inconclusive);
    assert!(timed_out.evidence.detail.contains("timed out"));

    let large_fixture = Fixture::new("#!/bin/sh\nyes x | head -c 4096\n");
    let output_bound_runner =
        CapabilityProbeRunner::with_limits(Duration::from_secs(1), 128).unwrap();
    let oversized = output_bound_runner
        .probe(
            &large_fixture.context(),
            &CapabilityProbeSpec::CommandVersion {
                tool: CapabilityToolId::Devtool,
            },
        )
        .await;
    assert_eq!(oversized.status, CapabilityProbeStatus::Inconclusive);
    assert!(oversized.evidence.detail.contains("safety bound"));

    let stale_fixture = Fixture::new("#!/bin/sh\necho ok\n");
    let context = stale_fixture.context();
    fs::remove_file(&stale_fixture.tool).unwrap();
    let stale = CapabilityProbeRunner::default()
        .probe(
            &context,
            &CapabilityProbeSpec::Executable {
                tool: CapabilityToolId::Devtool,
            },
        )
        .await;
    assert_eq!(stale.status, CapabilityProbeStatus::Inconclusive);
}
