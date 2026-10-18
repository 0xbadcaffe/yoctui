use super::*;
use std::{fs, os::unix::fs::PermissionsExt};

#[tokio::test]
async fn generated_setup_imports_from_root_or_build_and_sources_without_arguments() {
    let root = std::env::temp_dir().join(format!("yoctui-generated-setup-{}", std::process::id()));
    let build = root.join("build");
    fs::create_dir_all(build.join("conf")).unwrap();
    fs::write(build.join("conf/local.conf"), "").unwrap();
    fs::write(build.join("conf/bblayers.conf"), "").unwrap();
    let init = build.join("init-build-env");
    fs::write(
        &init,
        "test \"$#\" -eq 0 || return 1\nexport YOCTUI_GENERATED_SETUP=yes\n",
    )
    .unwrap();
    fs::set_permissions(&init, fs::Permissions::from_mode(0o644)).unwrap();
    let profile = discover_build_environment(&root, None).unwrap();
    assert_eq!(profile, discover_build_environment(&build, None).unwrap());
    assert_eq!(profile.init_script, init);
    let initialized = crate::BuildEnvironmentAdapter::default()
        .initialize(profile)
        .await
        .unwrap();
    assert_eq!(
        initialized
            .environment
            .get("YOCTUI_GENERATED_SETUP")
            .map(String::as_str),
        Some("yes")
    );
    assert!(std::env::var_os("YOCTUI_GENERATED_SETUP").is_none());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_sibling_source_and_invalid_setups_remain_distinct() {
    let root = std::env::temp_dir().join(format!("yoctui-discovery-legacy-{}", std::process::id()));
    let source = root.join("poky");
    let build = root.join("build");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(build.join("conf")).unwrap();
    fs::write(source.join("oe-init-build-env"), "").unwrap();
    fs::write(build.join("conf/local.conf"), "").unwrap();
    assert!(discover_build_environment(&build, Some(&source)).is_none());
    fs::write(build.join("conf/bblayers.conf"), "").unwrap();
    assert!(discover_build_environment(&build, None).is_none());
    assert_eq!(
        discover_build_environment(&build, Some(&source))
            .unwrap()
            .source_dir,
        source
    );
    fs::remove_dir_all(root).unwrap();
}
