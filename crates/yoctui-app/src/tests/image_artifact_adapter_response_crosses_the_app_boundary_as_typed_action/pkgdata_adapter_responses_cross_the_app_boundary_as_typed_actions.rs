use super::*;

#[test]
fn pkgdata_adapter_responses_cross_the_app_boundary_as_typed_actions() {
    let inventory_request = PackageInventoryRequest { generation: 11 };
    let package = PackageSummary {
        identity: PackageIdentity::new("busybox"),
        recipe: PackageField::Available("busybox".into()),
        provider: PackageField::Unavailable,
        version: PackageField::Available("1.37.0-r0".into()),
        installed_size_bytes: PackageField::Available(1_024),
        license: PackageField::Available("GPL-2.0-only".into()),
        image_membership: PackageField::Unavailable,
    };
    let event: BackendEvent = yoctui_bitbake::PackageInventoryResponse {
        request: inventory_request,
        packages: vec![package.clone()],
        limitations: vec!["provider path unavailable".into()],
    }
    .into();
    assert_eq!(
        model_action_from_backend_event(event),
        Some(Action::PackageInventoryPartial {
            request: inventory_request,
            packages: vec![package],
            limitations: vec!["provider path unavailable".into()],
        })
    );

    let request = PackageDetailRequest {
        identity: PackageIdentity::new("busybox"),
        generation: 4,
    };
    let detail = PackageDetail {
        identity: request.identity.clone(),
        files: PackageField::Available(vec!["/bin/busybox".into()]),
        runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
        reverse_dependencies: PackageField::Available(Vec::new()),
    };
    let event: BackendEvent = yoctui_bitbake::PackageDetailResponse {
        request: request.clone(),
        detail: detail.clone(),
        limitations: Vec::new(),
    }
    .into();
    assert_eq!(
        model_action_from_backend_event(event),
        Some(Action::PackageDetailLoaded { request, detail })
    );
}
