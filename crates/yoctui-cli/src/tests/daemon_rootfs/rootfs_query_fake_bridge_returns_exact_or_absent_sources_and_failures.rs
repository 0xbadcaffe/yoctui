use super::*;

#[tokio::test]
async fn rootfs_query_fake_bridge_returns_exact_or_absent_sources_and_failures() {
    for mode in ["ok", "absent", "error"] {
        let (build, query, compatibility, mut environment) = fixture();
        environment.insert("ROOTFS_TEST_MODE".into(), mode.into());
        let (_cancel, cancelled) = oneshot::channel();
        let result = acquire(
            query.clone(),
            build.clone(),
            compatibility,
            environment,
            cancelled,
            Duration::from_secs(2),
        )
        .await;
        if mode == "error" {
            assert!(result.unwrap_err().to_string().contains("fixture_error"));
        } else {
            let sources = result.unwrap();
            assert_eq!(sources.query, query);
            assert_eq!(
                sources.image_rootfs,
                (mode == "ok").then(|| build.join("retained-rootfs").display().to_string())
            );
        }
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
}
