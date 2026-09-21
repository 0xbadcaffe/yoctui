use super::*;

#[test]
fn pkgdata_workspace_routes_navigation_refresh_detail_and_contextual_actions() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        file: Some("/layers/meta/recipes-core/busybox.bb".into()),
        ..Recipe::default()
    });
    assert_eq!(
        update(&mut app, Action::Open(Screen::Packages)),
        Some(Effect::GetPackageInventory(PackageInventoryRequest {
            generation: 1
        }))
    );
    assert_eq!(app.screen, Screen::Packages);
    assert_eq!(NAVIGATOR_SCREENS[app.navigator_selection], Screen::Packages);
    assert_eq!(
        update(&mut app, Action::CancelPackageOperation),
        Some(Effect::CancelPackageOperation)
    );
    let request = PackageInventoryRequest { generation: 1 };
    let _ = update(
        &mut app,
        Action::PackageInventoryLoaded {
            request,
            packages: vec![
                package_summary("busybox", "busybox"),
                package_summary("init", "init"),
                package_summary("libc6", "glibc"),
            ],
        },
    );
    assert_eq!(
        update(&mut app, Action::BeginSelectedPackageDetail),
        Some(Effect::GetPackageDetail(PackageDetailRequest {
            identity: PackageIdentity::new("busybox"),
            generation: 2,
        }))
    );
    let detail_request = PackageDetailRequest {
        identity: PackageIdentity::new("busybox"),
        generation: 2,
    };
    let _ = update(
        &mut app,
        Action::PackageDetailLoaded {
            request: detail_request.clone(),
            detail: PackageDetail {
                identity: detail_request.identity,
                files: PackageField::Available(vec!["/bin/busybox".into()]),
                runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
                reverse_dependencies: PackageField::Available(vec![PackageIdentity::new("init")]),
            },
        },
    );
    assert_eq!(
        app.selected_package_dependency(),
        Some(&PackageIdentity::new("libc6"))
    );
    let _ = update(&mut app, Action::TogglePackageDependencyKind);
    assert_eq!(
        app.selected_package_dependency(),
        Some(&PackageIdentity::new("init"))
    );
    assert_eq!(
        update(&mut app, Action::OpenSelectedPackageDependency),
        Some(Effect::GetPackageDetail(PackageDetailRequest {
            identity: PackageIdentity::new("init"),
            generation: 3,
        }))
    );
    assert_eq!(app.package_selection, Some(PackageIdentity::new("init")));
    let _ = update(&mut app, Action::BackPackageNavigation);
    assert_eq!(app.package_selection, Some(PackageIdentity::new("busybox")));

    assert_eq!(
        update(&mut app, Action::OpenSelectedPackageProvider),
        Some(Effect::OpenInEditor(
            "/layers/meta/recipes/busybox.bb".into()
        ))
    );
    let _ = update(&mut app, Action::OpenSelectedPackageRecipe);
    assert_eq!(app.screen, Screen::Recipes);
    assert_eq!(app.recipe_selection, 0);

    app.package_details.clear();
    app.screen = Screen::Packages;
    assert_eq!(
        update(&mut app, Action::RefreshPackageInventory),
        Some(Effect::GetPackageInventory(PackageInventoryRequest {
            generation: 4
        }))
    );
}
