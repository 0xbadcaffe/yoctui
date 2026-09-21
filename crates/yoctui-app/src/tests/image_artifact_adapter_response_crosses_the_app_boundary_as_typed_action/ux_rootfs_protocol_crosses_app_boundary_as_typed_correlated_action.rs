use super::*;

#[test]
fn ux_rootfs_protocol_crosses_app_boundary_as_typed_correlated_action() {
    use yoctui_protocol::rootfs::{
        ROOTFS_COMPOSITION_SCHEMA_VERSION, RootfsAuthorityData, RootfsCompositionData,
        RootfsCompositionRequestData, RootfsEntryData, RootfsEntryKindData,
        RootfsImageIdentityData, RootfsInstalledPackageData,
    };

    let data = RootfsCompositionData {
        schema_version: ROOTFS_COMPOSITION_SCHEMA_VERSION,
        request: RootfsCompositionRequestData {
            generation: 4,
            image: RootfsImageIdentityData {
                machine: "qemux86-64".into(),
                image: "core-image-minimal".into(),
                path: "/build/tmp/deploy/images/qemux86-64/image.ext4".into(),
            },
        },
        installed_packages: RootfsAuthorityData::Available {
            records: vec![RootfsInstalledPackageData {
                name: "busybox".into(),
                recipe: Some("busybox".into()),
                category: "base".into(),
                installed_size_bytes: 1_024,
                file_count: 12,
            }],
        },
        filesystem_entries: RootfsAuthorityData::Partial {
            records: vec![RootfsEntryData {
                path: "/bin/busybox".into(),
                kind: RootfsEntryKindData::RegularFile,
                size_bytes: 1_024,
                package: Some("busybox".into()),
            }],
            limitations: vec!["hard-link count unavailable".into()],
        },
        limitations: vec!["manifest versions unavailable".into()],
    };
    let event = backend_event_from_rootfs_data(data).unwrap();
    let action = model_action_from_backend_event(event).unwrap();
    let Action::RootfsCompositionPartial {
        request,
        composition,
        limitations,
    } = action
    else {
        panic!("expected typed partial rootfs composition action")
    };
    assert_eq!(request.generation, 4);
    assert_eq!(request.image.image, "core-image-minimal");
    assert_eq!(
        composition.package_inventory().unwrap().packages[0].identity,
        yoctui_model::PackageIdentity::new("busybox")
    );
    assert_eq!(
        composition.filesystem_tree().unwrap().entries[0].kind,
        yoctui_model::RootfsEntryKind::RegularFile
    );
    assert_eq!(limitations, ["manifest versions unavailable"]);
}
