use super::*;

#[test]
fn rootfs_runtime_reverse_rejects_unsafe_missing_and_conflicting_mappings() {
    use std::os::unix::fs::symlink;
    let (build, _, sources) = fixture();
    let pkgdata = sources.pkgdata_directory.unwrap();
    let reverse = pkgdata.join("runtime-reverse");
    fs::create_dir(&reverse).unwrap();
    let runtime = pkgdata.join("runtime");
    fs::write(
        runtime.join("internal"),
        "PKG:internal: renamed\nPKGSIZE:internal: 8\nFILES_INFO:internal: {}\n",
    )
    .unwrap();
    for (name, target) in [
        ("absolute", runtime.join("internal")),
        ("escape", PathBuf::from("../../outside")),
        ("nested", PathBuf::from("../runtime/sub/internal")),
        ("dangling", PathBuf::from("../runtime/missing")),
        ("loop", PathBuf::from("loop")),
        ("wrong-pkg", PathBuf::from("../runtime/internal")),
    ] {
        symlink(target, reverse.join(name)).unwrap();
        let mut bytes = 0;
        assert!(
            read_installed_pkgdata(&pkgdata, name, &mut bytes).is_err(),
            "{name}"
        );
    }
    // A reverse entry cannot itself be an arbitrary regular metadata file.
    fs::write(reverse.join("regular"), "PKGSIZE: 99\n").unwrap();
    assert!(read_installed_pkgdata(&pkgdata, "regular", &mut 0).is_err());
    symlink("internal", runtime.join("chain")).unwrap();
    symlink("../runtime/chain", reverse.join("chain")).unwrap();
    assert!(read_installed_pkgdata(&pkgdata, "chain", &mut 0).is_err());
    // A conflicting direct record does not override the final-name index.
    fs::write(runtime.join("renamed"), "PKG: different\nPKGSIZE: 999\n").unwrap();
    symlink("../runtime/internal", reverse.join("renamed")).unwrap();
    assert_eq!(
        read_installed_pkgdata(&pkgdata, "renamed", &mut 0)
            .unwrap()
            .installed_size,
        Some(8)
    );
    // Mapped aliases cannot count the same record under a second identity.
    symlink("../runtime/internal", reverse.join("alias")).unwrap();
    assert!(read_installed_pkgdata(&pkgdata, "alias", &mut 0).is_err());
    fs::write(
        runtime.join("internal"),
        "PKG:internal: renamed\nPKG:internal: conflict\nPKG:internal: renamed\n",
    )
    .unwrap();
    assert!(read_installed_pkgdata(&pkgdata, "renamed", &mut 0).is_err());
    fs::write(runtime.join("internal"), "PKGSIZE:internal: 8\n").unwrap();
    assert!(read_installed_pkgdata(&pkgdata, "renamed", &mut 0).is_err());
    fs::remove_dir_all(build).unwrap();
}
