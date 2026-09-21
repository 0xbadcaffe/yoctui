use super::*;

#[test]
fn scans_only_supported_platform_artifacts_without_following_symlinks() {
    let root = std::env::temp_dir().join(format!("yoctui-platform-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("arch/arm/boot/dts")).unwrap();
    fs::write(root.join(".config"), "CONFIG_TEST=y\n").unwrap();
    fs::write(root.join("arch/arm/boot/dts/board.dts"), "/dts-v1/;\n").unwrap();
    fs::write(root.join("ignored.txt"), "ignored\n").unwrap();
    let scan = PlatformArtifactAdapter.scan(vec![root.clone()]).unwrap();
    assert_eq!(scan.files.len(), 2);
    assert!(
        scan.files
            .iter()
            .any(|file| file.kind == PlatformFileKind::DotConfig)
    );
    assert!(
        scan.files
            .iter()
            .any(|file| file.kind == PlatformFileKind::Dts)
    );
    fs::remove_dir_all(root).unwrap();
}
