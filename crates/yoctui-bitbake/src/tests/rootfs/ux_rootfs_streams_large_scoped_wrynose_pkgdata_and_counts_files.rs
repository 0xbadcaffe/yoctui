use super::*;

#[test]
fn ux_rootfs_streams_large_scoped_wrynose_pkgdata_and_counts_files() {
    let (build, _, sources) = fixture();
    let pkgdata = sources.pkgdata_directory.unwrap();
    let runtime = pkgdata.join("runtime");
    let files = (0..20_000)
        .map(|index| format!("\"/usr/src/kernel/file-{index}\":{{}}"))
        .collect::<Vec<_>>()
        .join(",");
    let content = format!(
        "PN: kernel-devsrc\nSECTION: kernel\nFILES_INFO:kernel-devsrc: {{{files}}}\nPKGSIZE:kernel-devsrc: 74306744\n"
    );
    assert!(content.len() > 256 * 1024);
    let path = runtime.join("kernel-devsrc");
    fs::write(&path, content).unwrap();

    let mut total = 0;
    let values = read_pkgdata(&path, &pkgdata, "kernel-devsrc", &mut total).unwrap();
    assert_eq!(values.recipe.as_deref(), Some("kernel-devsrc"));
    assert_eq!(values.category.as_deref(), Some("kernel"));
    assert_eq!(values.installed_size, Some(74_306_744));
    assert_eq!(values.file_count, Some(20_000));
    assert!(values.files_info_seen);
    assert_eq!(total, fs::metadata(path).unwrap().len());
    fs::remove_dir_all(build).unwrap();
}
