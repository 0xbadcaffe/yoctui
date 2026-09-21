use super::*;

#[test]
fn daemon_lifecycle_runtime_record_is_private_atomic_and_instance_guarded() {
    let paths = paths("record");
    let listener = DaemonListener::bind(&paths).unwrap();
    let record = DaemonRuntimeRecord {
        pid: std::process::id(),
        daemon_instance_id: DaemonInstanceId([4; 16]),
        started_unix_ms: SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis() as u64,
        boot_id: read_boot_id().unwrap(),
        executable: fs::read_link("/proc/self/exe").unwrap(),
    };
    write_runtime_record(&paths, &record).unwrap();
    assert_eq!(read_runtime_record(&paths).unwrap(), Some(record.clone()));
    assert_eq!(
        fs::symlink_metadata(runtime_record_path(&paths))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        SOCKET_MODE
    );
    assert_eq!(
        classify_runtime_record(&record, &record.boot_id),
        RuntimeRecordState::Current
    );
    assert_eq!(
        classify_runtime_record(&record, "different-boot"),
        RuntimeRecordState::Stale
    );
    assert!(remove_runtime_record(&paths, DaemonInstanceId([8; 16])).is_err());
    remove_runtime_record(&paths, record.daemon_instance_id).unwrap();
    drop(listener);
    fs::remove_dir_all(paths.directory.parent().unwrap()).unwrap();
}
