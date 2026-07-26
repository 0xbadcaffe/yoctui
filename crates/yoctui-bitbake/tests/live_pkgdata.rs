use std::path::PathBuf;

use yoctui_bitbake::PackageDataAdapter;
use yoctui_model::PackageInventoryRequest;

#[tokio::test]
#[ignore = "requires YOCTUI_LIVE_BUILD_DIR with real generated tmp/pkgdata"]
async fn pkgdata_adapter_live_smoke() {
    let build_dir = std::env::var_os("YOCTUI_LIVE_BUILD_DIR")
        .map(PathBuf::from)
        .expect("set YOCTUI_LIVE_BUILD_DIR to an initialized Yocto build with generated pkgdata");
    let expected_pkgdata = build_dir.join("tmp/pkgdata");
    assert!(
        expected_pkgdata.is_dir(),
        "generated pkgdata is missing at {}; build a target through do_package first",
        expected_pkgdata.display()
    );
    let response = PackageDataAdapter::new(build_dir)
        .inventory(PackageInventoryRequest { generation: 1 })
        .await
        .expect("real oe-pkgdata-util inventory query must succeed");
    assert!(
        !response.packages.is_empty(),
        "real generated pkgdata must expose at least one runtime package"
    );
    for package in &response.packages {
        package
            .identity
            .validate()
            .expect("live package identities must be typed and valid");
    }
}
