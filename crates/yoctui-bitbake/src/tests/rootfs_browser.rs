use super::*;

fn fixture() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "yoctui-rootfs-browser-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("etc")).unwrap();
    fs::write(root.join("etc/os-release"), "NAME=fixture\n").unwrap();
    fs::write(root.join(".hidden"), "hidden").unwrap();
    fs::canonicalize(root).unwrap()
}

#[test]
fn rootfs_browser_reads_lazy_sorted_entries_and_bounded_text_and_binary() {
    let root = fixture();
    let entries = scan_rootfs_browser_directory(&root, &root).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].is_dir);
    assert!(entries[1].is_hidden);
    let children = scan_rootfs_browser_directory(&root, &root.join("etc")).unwrap();
    assert_eq!(children[0].size, Some(13));
    assert_eq!(
        read_rootfs_browser_preview(&root, &children[0].path).unwrap(),
        ("NAME=fixture\n".into(), PreviewKind::Text, false)
    );
    fs::write(root.join("large"), vec![b'x'; 70_000]).unwrap();
    let (text, kind, truncated) = read_rootfs_browser_preview(&root, &root.join("large")).unwrap();
    assert_eq!(text.len(), 64 * 1024);
    assert_eq!(kind, PreviewKind::Text);
    assert!(truncated);
    fs::write(root.join("binary"), b"a\0b").unwrap();
    assert_eq!(
        read_rootfs_browser_preview(&root, &root.join("binary"))
            .unwrap()
            .1,
        PreviewKind::Binary
    );
    assert!(scan_rootfs_browser_directory(&root, &root.join("missing")).is_err());
    assert!(scan_rootfs_browser_directory(&root, root.parent().unwrap()).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn rootfs_browser_preserves_lstat_modes_owners_links_and_never_reads_special_files() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = fixture();
    let file = root.join("etc/os-release");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o4750)).unwrap();
    symlink("etc", root.join("directory-link")).unwrap();
    symlink("/etc/passwd", root.join("outside-link")).unwrap();
    symlink("missing", root.join("dangling-link")).unwrap();
    let fifo = std::ffi::CString::new(root.join("fifo").as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let entries = scan_rootfs_browser_directory(&root, &root).unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.path.ends_with("dangling-link"))
    );
    for name in ["outside-link", "directory-link", "dangling-link", "fifo"] {
        let entry = entries
            .iter()
            .find(|entry| entry.path.ends_with(name))
            .unwrap();
        assert!(!entry.is_dir);
        assert!(!entry.can_preview());
        assert!(read_rootfs_browser_preview(&root, &entry.path).is_err());
    }
    assert!(scan_rootfs_browser_directory(&root, &root.join("directory-link")).is_err());
    let entry = scan_rootfs_browser_directory(&root, &root.join("etc"))
        .unwrap()
        .remove(0);
    let attributes = entry.rootfs_metadata.unwrap();
    assert_eq!(attributes.mode, None);
    assert_eq!(attributes.permissions(), "??????????");
    assert_eq!(attributes.uid, None);
    assert_eq!(attributes.gid, None);
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
fn target_fixture() -> (std::path::PathBuf, rusqlite::Connection) {
    let work = fixture();
    let root = work.join("rootfs");
    fs::create_dir_all(root.join("etc")).unwrap();
    fs::create_dir_all(work.join("pseudo")).unwrap();
    fs::write(
        root.join("etc/passwd"),
        "root:x:0:0:root:/root:/bin/sh\napp:x:4242:73:app:/home/app:/bin/sh\n",
    )
    .unwrap();
    fs::write(root.join("etc/group"), "root:x:0:\nservice:x:73:\n").unwrap();
    let db = rusqlite::Connection::open(work.join("pseudo/files.db")).unwrap();
    db.execute_batch("CREATE TABLE files (id INTEGER PRIMARY KEY, path VARCHAR, dev INTEGER, ino INTEGER, uid INTEGER, gid INTEGER, mode INTEGER, rdev INTEGER, deleting INTEGER); CREATE INDEX file_paths ON files(path);").unwrap();
    (root, db)
}

#[cfg(unix)]
fn record(db: &rusqlite::Connection, path: &Path, uid: u32, gid: u32, mode: u32) {
    use std::os::unix::fs::MetadataExt;
    let metadata = fs::symlink_metadata(path).unwrap();
    db.execute(
        "INSERT INTO files(path,dev,ino,uid,gid,mode,rdev,deleting) VALUES (?1,?2,?3,?4,?5,?6,0,0)",
        rusqlite::params![
            path.to_str().unwrap(),
            metadata.dev() as i64,
            metadata.ino() as i64,
            uid,
            gid,
            mode
        ],
    )
    .unwrap();
}

#[cfg(unix)]
#[test]
fn rootfs_browser_reads_target_root_nonroot_unknown_names_and_modes_without_writes() {
    let (root, db) = target_fixture();
    let system = root.join("etc/passwd");
    record(&db, &system, 0, 0, 0o100440);
    let app = root.join("app-data");
    fs::write(&app, "application").unwrap();
    record(&db, &app, 4242, 73, 0o100640);
    let unnamed = root.join("numeric-only");
    fs::write(&unnamed, "data").unwrap();
    record(&db, &unnamed, 9999, 8888, 0o100600);
    let database = root.parent().unwrap().join("pseudo/files.db");
    let before = fs::read(&database).unwrap();
    let entries = scan_rootfs_browser_directory(&root, &root).unwrap();
    let attributes = entries
        .iter()
        .find(|entry| entry.path == app)
        .unwrap()
        .rootfs_metadata
        .as_ref()
        .unwrap();
    assert_eq!((attributes.uid, attributes.gid), (Some(4242), Some(73)));
    assert_eq!(attributes.owner.as_deref(), Some("app"));
    assert_eq!(attributes.group.as_deref(), Some("service"));
    assert!(
        attributes
            .listing(Some(11))
            .contains("0640 app(4242) service(73)")
    );
    let attributes = entries
        .iter()
        .find(|entry| entry.path == unnamed)
        .unwrap()
        .rootfs_metadata
        .as_ref()
        .unwrap();
    assert_eq!(
        (attributes.owner.as_deref(), attributes.group.as_deref()),
        (None, None)
    );
    assert!(attributes.listing(None).contains("0600 9999 8888"));
    let entries = scan_rootfs_browser_directory(&root, &root.join("etc")).unwrap();
    let attributes = entries
        .iter()
        .find(|entry| entry.path == system)
        .unwrap()
        .rootfs_metadata
        .as_ref()
        .unwrap();
    assert_eq!(attributes.owner.as_deref(), Some("root"));
    assert_eq!(attributes.group.as_deref(), Some("root"));
    assert_eq!(attributes.mode, Some(0o100440));
    assert_eq!(fs::read(&database).unwrap(), before);
    assert!(
        !root
            .parent()
            .unwrap()
            .join("pseudo/files.db-journal")
            .exists()
    );
    drop(db);
    fs::remove_dir_all(root.parent().unwrap()).unwrap();
}

#[cfg(unix)]
#[test]
fn rootfs_browser_rejects_stale_deleting_duplicate_invalid_and_locked_records() {
    let (root, db) = target_fixture();
    let file = root.join("data");
    fs::write(&file, "data").unwrap();
    let unavailable = || {
        let entries = scan_rootfs_browser_directory(&root, &root).unwrap();
        let attributes = entries
            .iter()
            .find(|entry| entry.path == file)
            .unwrap()
            .rootfs_metadata
            .as_ref()
            .unwrap();
        assert_eq!(
            (attributes.mode, attributes.uid, attributes.gid),
            (None, None, None)
        );
        assert!(!attributes.listing(None).contains("root(0)"));
        assert!(!attributes.listing(None).contains("bspguy-dev"));
    };
    record(&db, &file, 0, 0, 0o100644);
    for modification in [
        "ino=ino+1",
        "dev=dev+1",
        "deleting=1",
        "uid=-1",
        "gid=4294967296",
        "mode=0",
    ] {
        db.execute_batch(&format!("UPDATE files SET {modification}"))
            .unwrap();
        unavailable();
        db.execute_batch("DELETE FROM files").unwrap();
        record(&db, &file, 0, 0, 0o100644);
    }
    record(&db, &file, 4242, 73, 0o100644);
    unavailable();
    db.execute_batch("DELETE FROM files").unwrap();
    record(&db, &file, 0, 0, 0o100644);
    db.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let start = std::time::Instant::now();
    unavailable();
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
    db.execute_batch("ROLLBACK").unwrap();
    drop(db);
    fs::write(root.parent().unwrap().join("pseudo/files.db"), "not sqlite").unwrap();
    unavailable();
    fs::remove_dir_all(root.parent().unwrap()).unwrap();
}

#[cfg(unix)]
#[test]
fn rootfs_browser_never_resolves_host_or_symlink_special_oversized_ambiguous_accounts() {
    use std::os::unix::fs::symlink;
    let (root, db) = target_fixture();
    let file = root.join("data");
    fs::write(&file, "data").unwrap();
    record(&db, &file, 0, 0, 0o100644);
    let unnamed = || {
        let entries = scan_rootfs_browser_directory(&root, &root).unwrap();
        let attributes = entries
            .iter()
            .find(|entry| entry.path == file)
            .unwrap()
            .rootfs_metadata
            .as_ref()
            .unwrap();
        assert_eq!(attributes.uid, Some(0));
        assert_eq!(attributes.owner, None);
    };
    let passwd = root.join("etc/passwd");
    fs::remove_file(&passwd).unwrap();
    symlink("/etc/passwd", &passwd).unwrap();
    unnamed();
    fs::remove_file(&passwd).unwrap();
    let fifo = std::ffi::CString::new(passwd.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    unnamed();
    fs::remove_file(&passwd).unwrap();
    fs::write(&passwd, vec![b'x'; 1024 * 1024 + 1]).unwrap();
    unnamed();
    fs::write(
        &passwd,
        "root:x:0:0:r:/root:/bin/sh\nother:x:0:0:o:/root:/bin/sh\n",
    )
    .unwrap();
    unnamed();
    fs::write(&passwd, "invalid\x1b[31m:x:0:0:r:/root:/bin/sh\n").unwrap();
    unnamed();
    drop(db);
    let database = root.parent().unwrap().join("pseudo/files.db");
    fs::rename(&database, root.parent().unwrap().join("real.db")).unwrap();
    symlink("../real.db", &database).unwrap();
    let entries = scan_rootfs_browser_directory(&root, &root).unwrap();
    assert!(
        entries
            .iter()
            .all(|entry| entry.rootfs_metadata.as_ref().unwrap().uid.is_none())
    );
    fs::remove_dir_all(root.parent().unwrap()).unwrap();
}

#[cfg(unix)]
#[test]
#[ignore = "Requires an explicit genuine Yocto IMAGE_ROOTFS via YOCTUI_ROOTFS_SMOKE_ROOT"]
fn rootfs_browser_live_target_metadata_read_only_smoke() {
    let root = fs::canonicalize(
        std::env::var_os("YOCTUI_ROOTFS_SMOKE_ROOT").expect("set the exact IMAGE_ROOTFS"),
    )
    .unwrap();
    let database = root.parent().unwrap().join("pseudo/files.db");
    let before = fs::read(&database).unwrap();
    let entries = scan_rootfs_browser_directory(&root, &root.join("etc")).unwrap();
    for name in ["passwd", "group", "shadow"] {
        let entry = entries
            .iter()
            .find(|entry| entry.path.file_name().unwrap() == name)
            .unwrap();
        let attributes = entry.rootfs_metadata.as_ref().unwrap();
        assert!(attributes.uid.is_some() && attributes.gid.is_some() && attributes.mode.is_some());
        println!(
            "{}: {}",
            entry.path.display(),
            attributes.listing(entry.size)
        );
    }
    assert_eq!(
        before,
        fs::read(&database).unwrap(),
        "live Pseudo DB changed"
    );
}
