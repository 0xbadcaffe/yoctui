use super::*;

#[tokio::test]
async fn test_runner_spawn_retries_only_transient_text_file_busy() {
    let directory = TestDirectory::new("spawn-retry");
    let program = directory.path().join("test-runner");
    executable(&program, "#!/bin/sh\nexit 0\n");
    let writer = fs::OpenOptions::new().write(true).open(&program).unwrap();
    let release = tokio::spawn(async move {
        tokio::time::sleep(TEST_RUNNER_SPAWN_RETRY_DELAY + TEST_RUNNER_SPAWN_RETRY_DELAY).await;
        drop(writer);
    });
    let mut process = Command::new(&program);
    let mut child = spawn_test_runner_process(&mut process).await.unwrap();
    assert!(child.wait().await.unwrap().success());
    release.await.unwrap();

    let writer = fs::OpenOptions::new().write(true).open(&program).unwrap();
    let mut process = Command::new(&program);
    let error = match spawn_test_runner_process(&mut process).await {
        Ok(_) => panic!("write-held Testing executable unexpectedly spawned"),
        Err(error) => error,
    };
    assert!(is_transient_test_runner_spawn_error(&error));
    drop(writer);

    assert!(!is_transient_test_runner_spawn_error(
        &io::Error::from_raw_os_error(libc::EACCES)
    ));
}
