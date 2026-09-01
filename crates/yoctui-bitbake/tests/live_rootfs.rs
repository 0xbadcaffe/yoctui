use std::{env, path::PathBuf};

use yoctui_bitbake::{RootfsCompositionAdapter, RootfsCompositionSources};
use yoctui_model::{ImageArtifactIdentity, RootfsCompositionRequest};

#[tokio::test]
#[ignore = "requires an initialized build with an exact deployed image"]
async fn live_rootfs_composition_reads_current_generated_pkgdata() {
    let required_path = |name: &str| {
        PathBuf::from(
            env::var_os(name)
                .unwrap_or_else(|| panic!("set {name} to run the live rootfs composition test")),
        )
    };
    let build = required_path("YOCTUI_LIVE_BUILD_DIR");
    let manifest = required_path("YOCTUI_LIVE_IMAGE_MANIFEST");
    let pkgdata = required_path("YOCTUI_LIVE_PKGDATA_DIR");
    let artifact = required_path("YOCTUI_LIVE_IMAGE_ARTIFACT");
    let machine = env::var("YOCTUI_LIVE_MACHINE").expect("set YOCTUI_LIVE_MACHINE");
    let image = env::var("YOCTUI_LIVE_IMAGE").expect("set YOCTUI_LIVE_IMAGE");
    let image_rootfs = env::var_os("YOCTUI_LIVE_IMAGE_ROOTFS").map(PathBuf::from);
    let identity = ImageArtifactIdentity {
        machine,
        image,
        path: artifact,
    };
    let request = RootfsCompositionRequest {
        generation: 1,
        image: identity.clone(),
    };
    let response = RootfsCompositionAdapter::new(
        build,
        RootfsCompositionSources {
            image: identity,
            manifest: Some(manifest),
            pkgdata_directory: Some(pkgdata),
            image_rootfs,
        },
        1,
    )
    .scan(request)
    .await
    .expect("live generated rootfs composition must scan without a screen-level failure");

    let inventory = response
        .composition
        .package_inventory()
        .expect("the deployed image manifest and generated pkgdata must provide packages");
    assert!(!inventory.packages.is_empty());
    assert!(
        inventory
            .packages
            .iter()
            .any(|package| package.installed_size_bytes > 0 && package.file_count > 0),
        "at least one live package must retain exact size and file-count evidence"
    );
}
