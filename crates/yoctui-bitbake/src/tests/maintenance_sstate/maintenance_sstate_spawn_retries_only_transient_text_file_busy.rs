use super::*;

#[tokio::test]
async fn maintenance_sstate_spawn_retries_only_transient_text_file_busy() {
    let root = TestDirectory::new("spawn-retry");
    let executable = root.0.join("runner");
    write_executable(&executable, "#!/bin/sh\nexit 0\n");
    let writer = fs::OpenOptions::new()
        .write(true)
        .open(&executable)
        .unwrap();
    let release = tokio::spawn(async move {
        tokio::time::sleep(SPAWN_RETRY_DELAY + SPAWN_RETRY_DELAY).await;
        drop(writer);
    });
    let mut process = Command::new(&executable);
    let mut child = spawn_process(&mut process).await.unwrap();
    assert!(child.wait().await.unwrap().success());
    release.await.unwrap();

    let writer = fs::OpenOptions::new()
        .write(true)
        .open(&executable)
        .unwrap();
    let mut process = Command::new(&executable);
    let error = match spawn_process(&mut process).await {
        Ok(_) => panic!("write-held executable unexpectedly spawned"),
        Err(error) => error,
    };
    assert!(is_transient_spawn_error(&error));
    drop(writer);

    assert!(!is_transient_spawn_error(
        &std::io::Error::from_raw_os_error(libc::EACCES)
    ));
}
