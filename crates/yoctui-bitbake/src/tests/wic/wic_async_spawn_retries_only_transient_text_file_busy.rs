use super::*;

#[tokio::test]
async fn wic_async_spawn_retries_only_transient_text_file_busy() {
    let directory = fixture("spawn-retry");
    let program = directory.join("wic");
    executable(&program, "exit 0");
    let writer = fs::OpenOptions::new().write(true).open(&program).unwrap();
    let release = tokio::spawn(async move {
        tokio::time::sleep(WIC_SPAWN_RETRY_DELAY + WIC_SPAWN_RETRY_DELAY).await;
        drop(writer);
    });
    let mut command = Command::new(&program);
    let mut child = spawn_async_command(&mut command).await.unwrap();
    assert!(child.wait().await.unwrap().success());
    release.await.unwrap();

    let writer = fs::OpenOptions::new().write(true).open(&program).unwrap();
    let mut command = Command::new(&program);
    let error = match spawn_async_command(&mut command).await {
        Ok(_) => panic!("write-held executable unexpectedly spawned"),
        Err(error) => error,
    };
    assert!(is_transient_spawn_error(&error));
    drop(writer);

    assert!(!is_transient_spawn_error(
        &std::io::Error::from_raw_os_error(libc::EACCES)
    ));
    fs::remove_dir_all(directory).unwrap();
}
