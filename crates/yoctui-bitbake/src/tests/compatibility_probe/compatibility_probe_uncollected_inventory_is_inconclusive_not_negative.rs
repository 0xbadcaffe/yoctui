use super::*;

#[tokio::test]
async fn compatibility_probe_uncollected_inventory_is_inconclusive_not_negative() {
    let fixture = Fixture::new("#!/bin/sh\necho ok\n");
    let mut context = fixture.context();
    context.metadata_tasks = None;
    context.metadata_variables = None;
    context.backend_capabilities = None;
    context.configurations = None;
    for probe in [
        CapabilityProbeSpec::MetadataAnyTask {
            names: vec!["do_build".into()],
        },
        CapabilityProbeSpec::MetadataVariable {
            name: "MACHINE".into(),
        },
        CapabilityProbeSpec::BackendCapability {
            name: "workspace".into(),
        },
        CapabilityProbeSpec::Configuration {
            name: "ptest_enabled".into(),
        },
    ] {
        assert_eq!(
            CapabilityProbeRunner::default()
                .probe(&context, &probe)
                .await
                .status,
            CapabilityProbeStatus::Inconclusive
        );
    }
}
