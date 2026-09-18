use super::*;

#[cfg(unix)]
#[tokio::test]
async fn pkgdata_workspace_background_operation_binds_current_authority_and_reports_results() {
    use std::os::unix::fs::PermissionsExt;

    let directory = std::env::temp_dir().join(format!(
        "yoctui-pkgdata-workspace-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let build_dir = directory.join("build");
    let pkgdata_dir = build_dir.join("tmp/pkgdata");
    fs::create_dir_all(&pkgdata_dir).unwrap();
    let tool = directory.join("oe-pkgdata-util");
    let write_tool = |body: &str| {
        fs::write(&tool, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(&tool).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&tool, permissions).unwrap();
    };
    write_tool(
        r#"case "$3" in
list-pkgs) printf 'busybox\nlibc6\n' ;;
package-info) printf 'busybox 1.37.0-r0 busybox 1.37.0-r0 1024 "GPL-2.0-only"\nlibc6 2.40-r0 glibc 2.40-r0 4096 "GPL-2.0-or-later"\n' ;;
list-pkg-files) printf 'busybox:\n\t/bin/busybox\n' ;;
read-value) printf 'busybox libc6\nlibc6\n' ;;
*) exit 9 ;;
esac"#,
    );
    let compatibility = pkgdata_test_compatibility(&build_dir, &tool);
    let adapter = PackageDataAdapter::new(build_dir);
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Packages;
    app.workspace.variables.insert(
        "PKGDATA_DIR".into(),
        pkgdata_dir.to_string_lossy().into_owned(),
    );
    yoctui_model::install_workspace_compatibility(&mut app, compatibility).unwrap();
    let effect = update(&mut app, Action::BeginPackageInventory).unwrap();
    let mut operation = None;
    begin_package_operation(&mut app, &adapter, &mut operation, effect);
    tokio::time::timeout(Duration::from_secs(2), async {
        while operation.is_some() {
            poll_package_operation(&mut app, &mut operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        app.package_inventory,
        yoctui_model::PackageInventoryState::Partial { .. }
    ));
    assert_eq!(
        app.package_selection,
        Some(yoctui_model::PackageIdentity::new("busybox"))
    );

    let effect = update(&mut app, Action::BeginSelectedPackageDetail).unwrap();
    begin_package_operation(&mut app, &adapter, &mut operation, effect);
    tokio::time::timeout(Duration::from_secs(2), async {
        while operation.is_some() {
            poll_package_operation(&mut app, &mut operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        app.selected_package_detail(),
        Some(yoctui_model::PackageDetailState::Available { .. })
    ));

    write_tool("sleep 30");
    let effect = update(&mut app, Action::RefreshPackageInventory).unwrap();
    begin_package_operation(&mut app, &adapter, &mut operation, effect);
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        operation
            .as_ref()
            .is_some_and(|operation| operation.cancellation.cancel())
    );
    tokio::time::timeout(Duration::from_secs(2), async {
        while operation.is_some() {
            poll_package_operation(&mut app, &mut operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        app.package_inventory,
        yoctui_model::PackageInventoryState::Failed { ref message, .. }
            if message.contains("cancelled")
    ));
    fs::remove_dir_all(directory).unwrap();
}
