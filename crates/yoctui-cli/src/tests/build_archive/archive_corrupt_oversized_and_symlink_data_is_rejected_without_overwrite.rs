use super::*;

#[test]
fn archive_corrupt_oversized_and_symlink_data_is_rejected_without_overwrite() {
    let root = Root::new();
    save(&root.0, record(1)).unwrap();
    let file = root.0.join("yoctui/build-history/history.json");
    fs::write(&file, b"broken JSON").unwrap();
    assert!(save(&root.0, record(2)).is_err());
    assert_eq!(fs::read(&file).unwrap(), b"broken JSON");
    fs::OpenOptions::new()
        .write(true)
        .open(&file)
        .unwrap()
        .set_len((MAX_ARCHIVE_BYTES + 1) as u64)
        .unwrap();
    assert!(read(&root.0).is_err());
    fs::remove_file(&file).unwrap();
    std::os::unix::fs::symlink("/etc/passwd", &file).unwrap();
    assert!(read(&root.0).is_err());
}
