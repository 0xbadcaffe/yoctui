use super::*;

#[tokio::test]
async fn rootfs_query_cancellation_timeout_and_single_worker_bound_reap_bridge() {
    let (build, query, compatibility, mut environment) = fixture();
    environment.insert("ROOTFS_TEST_MODE".into(), "hang".into());
    let permit = Arc::new(Semaphore::new(1));
    let mut pending = PendingQuery::start(
        RequestId(1),
        query.clone(),
        query.daemon_instance_id,
        compatibility.clone(),
        environment.clone(),
        permit.clone(),
    )
    .unwrap();
    assert!(
        PendingQuery::start(
            RequestId(2),
            query.clone(),
            query.daemon_instance_id,
            compatibility.clone(),
            environment.clone(),
            permit.clone()
        )
        .is_err()
    );
    tokio::time::timeout(Duration::from_secs(3), async {
        while !build.join("query.pid").is_file() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    pending.shutdown().await;
    assert!(pending.try_result().unwrap().is_err());
    assert_eq!(permit.available_permits(), 1);
    let (_cancel, cancelled) = oneshot::channel();
    let result = acquire(
        query,
        build.clone(),
        compatibility,
        environment,
        cancelled,
        Duration::from_millis(100),
    )
    .await;
    assert!(result.unwrap_err().to_string().contains("timed out"));
    let pid: i32 = fs::read_to_string(build.join("query.pid"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        unsafe { libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG) },
        -1
    );
    fs::remove_dir_all(build).unwrap();
}
