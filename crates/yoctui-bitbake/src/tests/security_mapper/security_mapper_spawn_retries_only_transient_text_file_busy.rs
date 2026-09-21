use super::*;

#[tokio::test]
async fn security_mapper_spawn_retries_only_transient_text_file_busy() {
    let directory = TestDirectory::new("spawn-retry");
    let executable = directory.path().join("cve-check-map-pkgs");
    write_executable(&executable, "#!/bin/sh\nexit 0\n");

    let writer = fs::OpenOptions::new()
        .write(true)
        .open(&executable)
        .unwrap();
    let release = tokio::spawn(async move {
        tokio::time::sleep(SECURITY_MAPPER_SPAWN_RETRY_DELAY + SECURITY_MAPPER_SPAWN_RETRY_DELAY)
            .await;
        drop(writer);
    });
    let mut process = Command::new(&executable);
    let mut child = spawn_security_mapper_process(&mut process).await.unwrap();
    assert!(child.wait().await.unwrap().success());
    release.await.unwrap();

    let writer = fs::OpenOptions::new()
        .write(true)
        .open(&executable)
        .unwrap();
    let mut process = Command::new(&executable);
    let error = match spawn_security_mapper_process(&mut process).await {
        Ok(_) => panic!("write-held Security mapper executable unexpectedly spawned"),
        Err(error) => error,
    };
    assert!(is_transient_security_mapper_spawn_error(&error));
    drop(writer);

    assert!(!is_transient_security_mapper_spawn_error(
        &io::Error::from_raw_os_error(libc::EACCES)
    ));
}
