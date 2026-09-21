use super::*;

#[test]
fn rootfs_runtime_reverse_preserves_file_and_total_byte_limits() {
    let (build, _, sources) = fixture();
    let pkgdata = sources.pkgdata_directory.unwrap();
    fs::create_dir(pkgdata.join("runtime-reverse")).unwrap();
    std::os::unix::fs::symlink(
        "../runtime/busybox",
        pkgdata.join("runtime-reverse/busybox"),
    )
    .unwrap();
    let mut total_bytes = MAX_PKGDATA_TOTAL_BYTES;
    assert!(matches!(
        read_installed_pkgdata(&pkgdata, "busybox", &mut total_bytes),
        Err(RootfsCompositionAdapterError::ResourceLimit(_))
    ));
    fs::File::create(pkgdata.join("runtime/busybox"))
        .unwrap()
        .set_len(MAX_PKGDATA_FILE_BYTES + 1)
        .unwrap();
    assert!(matches!(
        read_installed_pkgdata(&pkgdata, "busybox", &mut 0),
        Err(RootfsCompositionAdapterError::ResourceLimit(_))
    ));
    fs::remove_dir_all(build).unwrap();
}
