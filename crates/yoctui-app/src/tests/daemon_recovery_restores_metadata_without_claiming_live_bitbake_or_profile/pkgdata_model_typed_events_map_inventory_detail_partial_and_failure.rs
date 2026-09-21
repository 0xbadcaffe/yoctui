use super::*;

#[test]
fn pkgdata_model_typed_events_map_inventory_detail_partial_and_failure() {
    let inventory_request = PackageInventoryRequest { generation: 7 };
    let package = PackageSummary {
        identity: PackageIdentity::new("busybox"),
        recipe: PackageField::Available("busybox".into()),
        provider: PackageField::Available("/layers/core/recipes-core/busybox.bb".into()),
        version: PackageField::Available("1.37.0".into()),
        installed_size_bytes: PackageField::Available(1_024),
        license: PackageField::Available("GPL-2.0-only".into()),
        image_membership: PackageField::Available(vec!["core-image-minimal".into()]),
    };
    assert_eq!(
        model_action_from_backend_event(BackendEvent::PackageInventory {
            request: inventory_request,
            packages: vec![package.clone()],
            limitations: Vec::new(),
        }),
        Some(Action::PackageInventoryLoaded {
            request: inventory_request,
            packages: vec![package],
        })
    );
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::PackageInventory {
            request: inventory_request,
            packages: Vec::new(),
            limitations: vec!["pkgdata directory is incomplete".into()],
        }),
        Some(Action::PackageInventoryPartial { .. })
    ));
    assert_eq!(
        model_action_from_backend_event(BackendEvent::PackageInventoryFailed {
            request: inventory_request,
            message: "pkgdata directory is missing".into(),
        }),
        Some(Action::PackageInventoryFailed {
            request: inventory_request,
            message: "pkgdata directory is missing".into(),
        })
    );

    let detail_request = PackageDetailRequest {
        identity: PackageIdentity::new("busybox"),
        generation: 3,
    };
    let detail = PackageDetail {
        identity: detail_request.identity.clone(),
        files: PackageField::Available(vec!["/bin/busybox".into()]),
        runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
        reverse_dependencies: PackageField::Unavailable,
    };
    assert!(matches!(
        model_action_from_backend_event(BackendEvent::PackageDetail {
            request: detail_request.clone(),
            detail: detail.clone(),
            limitations: vec!["reverse dependencies unavailable".into()],
        }),
        Some(Action::PackageDetailPartial { .. })
    ));
    assert_eq!(
        model_action_from_backend_event(BackendEvent::PackageDetailFailed {
            request: detail_request.clone(),
            message: "package was not found".into(),
        }),
        Some(Action::PackageDetailFailed {
            request: detail_request,
            message: "package was not found".into(),
        })
    );
}
