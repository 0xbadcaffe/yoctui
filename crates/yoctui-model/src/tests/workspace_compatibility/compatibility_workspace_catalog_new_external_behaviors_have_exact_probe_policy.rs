use super::*;

#[test]
fn compatibility_workspace_catalog_new_external_behaviors_have_exact_probe_policy() {
    let catalog = crate::CapabilityCatalog::builtin();
    for id in [
        CapabilityId::SdkPublish,
        CapabilityId::SdkNativeTools,
        CapabilityId::BitBakeSelftest,
        CapabilityId::TestImage,
        CapabilityId::TestSdk,
        CapabilityId::TestSdkExtensible,
        CapabilityId::Ptest,
        CapabilityId::QaTask,
        CapabilityId::BuildHistoryCompare,
        CapabilityId::SstateReadiness,
        CapabilityId::SstateCleanup,
        CapabilityId::PrservManagement,
        CapabilityId::BuildCompare,
        CapabilityId::GitArchive,
    ] {
        let entry = catalog
            .entry(id)
            .expect("workspace capability is cataloged");
        assert!(!entry.probes.is_empty(), "{id} lacks probe policy");
        assert!(
            !entry.required_tools.is_empty() || !entry.required_metadata.is_empty(),
            "{id} lacks an authoritative requirement"
        );
        assert!(!entry.preferred.id.is_empty(), "{id} lacks implementation");
    }
    assert_eq!(
        catalog
            .entry(CapabilityId::SdkPublish)
            .unwrap()
            .required_tools,
        [crate::CapabilityToolId::OePublishSdk]
    );
    assert_eq!(
        catalog
            .entry(CapabilityId::BitBakeSelftest)
            .unwrap()
            .required_tools,
        [crate::CapabilityToolId::BitBakeSelftest]
    );
}
