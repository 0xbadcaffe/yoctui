use super::*;

#[test]
fn pkgdata_model_reducer_correlates_inventory_states_search_and_selection() {
    let mut app = App::new(10, 1_000);
    assert_eq!(
        update(&mut app, Action::BeginPackageInventory),
        Some(Effect::GetPackageInventory(PackageInventoryRequest {
            generation: 1
        }))
    );
    let request = PackageInventoryRequest { generation: 1 };
    let _ = update(
        &mut app,
        Action::PackageInventoryLoaded {
            request: PackageInventoryRequest { generation: 99 },
            packages: vec![package_summary("stale", "stale")],
        },
    );
    assert_eq!(
        app.package_inventory,
        PackageInventoryState::Loading { request }
    );
    let mut invalid_field = package_summary("libc6", "glibc");
    invalid_field.provider = PackageField::Available("relative.bb".into());
    let _ = update(
        &mut app,
        Action::PackageInventoryLoaded {
            request,
            packages: vec![
                package_summary("busybox", "busybox"),
                invalid_field,
                package_summary("busybox", "zzz"),
            ],
        },
    );
    assert!(matches!(
        app.package_inventory,
        PackageInventoryState::Partial { .. }
    ));
    assert_eq!(app.package_selection, Some(PackageIdentity::new("busybox")));

    let _ = update(&mut app, Action::BeginPackageSearch);
    let _ = update(&mut app, Action::AppendPackageQuery('G'));
    let _ = update(&mut app, Action::AppendPackageQuery('L'));
    assert_eq!(app.package_selection, Some(PackageIdentity::new("libc6")));
    assert_eq!(app.filtered_packages().len(), 1);
    let _ = update(&mut app, Action::BackspacePackageQuery);
    let _ = update(&mut app, Action::FinishPackageSearch);
    assert!(!app.package_searching);

    let _ = update(&mut app, Action::BeginPackageInventory);
    let request = PackageInventoryRequest { generation: 2 };
    let _ = update(
        &mut app,
        Action::PackageInventoryLoaded {
            request,
            packages: vec![package_summary("libc6", "glibc")],
        },
    );
    assert_eq!(app.package_selection, Some(PackageIdentity::new("libc6")));

    let _ = update(&mut app, Action::BeginPackageInventory);
    let request = PackageInventoryRequest { generation: 3 };
    let _ = update(
        &mut app,
        Action::PackageInventoryLoaded {
            request,
            packages: Vec::new(),
        },
    );
    assert_eq!(
        app.package_inventory,
        PackageInventoryState::AvailableEmpty { request }
    );
    assert_eq!(app.package_selection, None);

    let _ = update(&mut app, Action::BeginPackageInventory);
    let request = PackageInventoryRequest { generation: 4 };
    let _ = update(
        &mut app,
        Action::PackageInventoryFailed {
            request,
            message: "pkgdata missing".into(),
        },
    );
    assert_eq!(
        app.package_inventory,
        PackageInventoryState::Failed {
            request,
            message: "pkgdata missing".into()
        }
    );
}
