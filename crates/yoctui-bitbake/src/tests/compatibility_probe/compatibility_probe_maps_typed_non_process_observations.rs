use super::*;

#[tokio::test]
async fn compatibility_probe_maps_typed_non_process_observations() {
    let fixture = Fixture::new("#!/bin/sh\necho ok\n");
    let context = fixture.context();
    let runner = CapabilityProbeRunner::default();
    for probe in [
        CapabilityProbeSpec::MetadataAnyTask {
            names: vec!["create_spdx".into()],
        },
        CapabilityProbeSpec::MetadataVariable {
            name: "MACHINE".into(),
        },
        CapabilityProbeSpec::BackendCapability {
            name: "getvar".into(),
        },
        CapabilityProbeSpec::ProtocolCapability {
            name: "state_snapshots".into(),
        },
        CapabilityProbeSpec::Artifact { kind: "wic".into() },
        CapabilityProbeSpec::Configuration {
            name: "buildhistory".into(),
        },
    ] {
        assert_eq!(
            runner.probe(&context, &probe).await.status,
            CapabilityProbeStatus::Positive
        );
    }
    assert_eq!(
        runner
            .probe(
                &context,
                &CapabilityProbeSpec::MetadataVariable {
                    name: "ABSENT".into()
                },
            )
            .await
            .status,
        CapabilityProbeStatus::Negative
    );
}
