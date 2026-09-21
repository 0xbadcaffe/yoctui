use super::*;

#[test]
fn pkgdata_model_detail_states_and_dependency_navigation_are_exact() {
    let mut app = App::new(10, 1_000);
    let inventory_request = PackageInventoryRequest { generation: 1 };
    app.package_inventory = PackageInventoryState::Loading {
        request: inventory_request,
    };
    let _ = update(
        &mut app,
        Action::PackageInventoryLoaded {
            request: inventory_request,
            packages: vec![
                package_summary("busybox", "busybox"),
                package_summary("libc6", "glibc"),
                package_summary("init", "init"),
            ],
        },
    );
    assert_eq!(
        update(&mut app, Action::BeginSelectedPackageDetail),
        Some(Effect::GetPackageDetail(PackageDetailRequest {
            identity: PackageIdentity::new("busybox"),
            generation: 1,
        }))
    );
    let request = PackageDetailRequest {
        identity: PackageIdentity::new("busybox"),
        generation: 1,
    };
    let detail = PackageDetail {
        identity: request.identity.clone(),
        files: PackageField::Available(vec!["/bin/busybox".into()]),
        runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
        reverse_dependencies: PackageField::Available(vec![PackageIdentity::new("init")]),
    };
    let _ = update(
        &mut app,
        Action::PackageDetailLoaded {
            request: PackageDetailRequest {
                generation: 99,
                ..request.clone()
            },
            detail: detail.clone(),
        },
    );
    assert!(matches!(
        app.selected_package_detail(),
        Some(PackageDetailState::Loading { .. })
    ));
    let _ = update(
        &mut app,
        Action::PackageDetailPartial {
            request: request.clone(),
            detail,
            limitations: vec!["license unavailable".into()],
        },
    );
    assert!(matches!(
        app.selected_package_detail(),
        Some(PackageDetailState::Partial { .. })
    ));

    let _ = update(
        &mut app,
        Action::OpenPackageDependency {
            identity: PackageIdentity::new("libc6"),
            reverse: false,
        },
    );
    assert_eq!(app.package_selection, Some(PackageIdentity::new("libc6")));
    app.package_selection = Some(PackageIdentity::new("busybox"));
    let _ = update(
        &mut app,
        Action::OpenPackageDependency {
            identity: PackageIdentity::new("init"),
            reverse: true,
        },
    );
    assert_eq!(app.package_selection, Some(PackageIdentity::new("init")));
    app.package_selection = Some(PackageIdentity::new("busybox"));
    let _ = update(
        &mut app,
        Action::OpenPackageDependency {
            identity: PackageIdentity::new("not-present"),
            reverse: false,
        },
    );
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("not in the current typed detail"))
    );

    app.package_selection = Some(PackageIdentity::new("libc6"));
    let effect = update(&mut app, Action::BeginSelectedPackageDetail).unwrap();
    let Effect::GetPackageDetail(empty_request) = effect else {
        panic!("expected package detail effect");
    };
    let empty = PackageDetail {
        identity: empty_request.identity.clone(),
        files: PackageField::Available(Vec::new()),
        runtime_dependencies: PackageField::Available(Vec::new()),
        reverse_dependencies: PackageField::Available(Vec::new()),
    };
    let _ = update(
        &mut app,
        Action::PackageDetailLoaded {
            request: empty_request.clone(),
            detail: empty,
        },
    );
    assert_eq!(
        app.package_details.get(&empty_request.identity),
        Some(&PackageDetailState::AvailableEmpty {
            request: empty_request.clone()
        })
    );

    app.package_selection = Some(PackageIdentity::new("init"));
    let Effect::GetPackageDetail(failed_request) =
        update(&mut app, Action::BeginSelectedPackageDetail).unwrap()
    else {
        panic!("expected package detail effect");
    };
    let _ = update(
        &mut app,
        Action::PackageDetailFailed {
            request: failed_request.clone(),
            message: "tool failed".into(),
        },
    );
    assert_eq!(
        app.package_details.get(&failed_request.identity),
        Some(&PackageDetailState::Failed {
            request: failed_request,
            message: "tool failed".into()
        })
    );
}
