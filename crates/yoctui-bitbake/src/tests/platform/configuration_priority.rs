use super::*;

#[test]
fn platform_scan_keeps_later_root_configuration_when_source_exhausts_file_budget() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-platform-budget-{}-{}",
        std::process::id(),
        yoctui_utils::unix_ms()
    ));
    let source = root.join("a-source");
    let build = root.join("z-build");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&build).unwrap();
    for index in 0..MAX_FILES + 1 {
        fs::write(source.join(format!("board-{index:05}.dts")), "/dts-v1/;\n").unwrap();
    }
    let config = build.join(".config");
    fs::write(&config, "CONFIG_TEST=y\n").unwrap();
    let scan = PlatformArtifactAdapter
        .scan(vec![source.clone(), build.clone(), source.clone()])
        .unwrap();
    assert_eq!(scan.files.len(), MAX_FILES);
    assert_eq!(scan.roots.len(), 2);
    let selected = scan.files.iter().find(|file| file.path == config).unwrap();
    assert_eq!(selected.kind, PlatformFileKind::DotConfig);
    assert_eq!(selected.root, build);
    assert_eq!(selected.size_bytes, 14);
    assert!(
        scan.limitations
            .iter()
            .any(|text| text.contains("Scan stopped"))
    );
    assert!(
        scan.files
            .windows(2)
            .all(|pair| pair[0].path < pair[1].path)
    );
    let single_root = PlatformArtifactAdapter.scan(vec![source]).unwrap();
    assert_eq!(single_root.files.len(), MAX_FILES);
    assert_eq!(
        single_root
            .limitations
            .iter()
            .filter(|text| text.contains("Scan stopped"))
            .count(),
        1,
        "even a flat single-root budget stop must report partial discovery exactly once"
    );
    assert!(
        single_root
            .files
            .iter()
            .all(|file| file.kind == PlatformFileKind::Dts)
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn platform_configuration_preflight_does_not_follow_file_or_directory_symlinks() {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!(
        "yoctui-platform-links-{}-{}",
        std::process::id(),
        yoctui_utils::unix_ms()
    ));
    let source = root.join("source");
    let outside = root.join("outside");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join(".config"), "CONFIG_OUTSIDE=y\n").unwrap();
    fs::write(outside.join("escape.dts"), "/dts-v1/;\n").unwrap();
    fs::write(source.join("inside.dts"), "/dts-v1/;\n").unwrap();
    symlink(outside.join(".config"), source.join(".config")).unwrap();
    symlink(outside.join("escape.dts"), source.join("link.dts")).unwrap();
    symlink(&outside, source.join("link-dir")).unwrap();
    let scan = PlatformArtifactAdapter.scan(vec![source.clone()]).unwrap();
    assert_eq!(scan.files.len(), 1);
    assert_eq!(scan.files[0].path, source.join("inside.dts"));
    assert_eq!(scan.files[0].kind, PlatformFileKind::Dts);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn platform_preflight_configs_are_not_duplicated_by_overlapping_root_recursion() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-platform-overlap-{}-{}",
        std::process::id(),
        yoctui_utils::unix_ms()
    ));
    let nested = root.join("build");
    fs::create_dir_all(&nested).unwrap();
    for path in [root.join(".config"), nested.join(".config")] {
        fs::write(path, "CONFIG_TEST=y\n").unwrap();
    }
    let scan = PlatformArtifactAdapter
        .scan(vec![root.clone(), nested])
        .unwrap();
    assert_eq!(scan.files.len(), 2);
    assert!(
        scan.files
            .iter()
            .all(|file| file.kind == PlatformFileKind::DotConfig)
    );
    assert!(
        scan.files
            .windows(2)
            .all(|pair| pair[0].path < pair[1].path)
    );
    assert!(
        scan.limitations
            .iter()
            .any(|text| text.contains("Overlapping"))
    );
    fs::remove_dir_all(root).unwrap();
}
