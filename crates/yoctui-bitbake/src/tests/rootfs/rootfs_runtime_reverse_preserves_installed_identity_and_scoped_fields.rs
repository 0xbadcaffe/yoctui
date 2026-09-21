use super::*;

#[tokio::test]
async fn rootfs_runtime_reverse_preserves_installed_identity_and_scoped_fields() {
    let (build, request, sources) = fixture();
    let pkgdata = sources.pkgdata_directory.as_ref().unwrap();
    fs::create_dir(pkgdata.join("runtime-reverse")).unwrap();
    fs::write(
        sources.manifest.as_ref().unwrap(),
        "libblkid1 arm1176jzs 1.0\nlibblkid1 arm1176jzs 1.0\nbusybox arm1176jzs 1.0\n",
    )
    .unwrap();
    fs::write(
            pkgdata.join("runtime/util-linux-libblkid"),
            "PN: util-linux\nPKG:util-linux-libblkid: libblkid1\nSECTION: base\nPKGSIZE:util-linux-libblkid: 354189\nFILES_INFO:util-linux-libblkid: {\"/usr/lib/libblkid.so.1\":17,\"/usr/lib/libblkid.so.1.1.0\":354172}\n",
        ).unwrap();
    std::os::unix::fs::symlink(
        "../runtime/util-linux-libblkid",
        pkgdata.join("runtime-reverse/libblkid1"),
    )
    .unwrap();
    let response = RootfsCompositionAdapter::new(build.clone(), sources, 4)
        .scan(request)
        .await
        .unwrap();
    assert!(matches!(
        response.composition.installed_packages,
        RootfsAuthority::Available(_)
    ));
    let packages = &response.composition.package_inventory().unwrap().packages;
    assert_eq!(packages.len(), 2);
    let renamed = packages
        .iter()
        .find(|p| p.identity.name == "libblkid1")
        .unwrap();
    assert_eq!(renamed.recipe.as_deref(), Some("util-linux"));
    assert_eq!(renamed.installed_size_bytes, 354189);
    assert_eq!(renamed.file_count, 2);
    fs::remove_dir_all(build).unwrap();
}
