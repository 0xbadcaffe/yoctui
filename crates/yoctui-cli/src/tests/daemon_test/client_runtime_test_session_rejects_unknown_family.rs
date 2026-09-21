use super::*;

#[test]
fn client_runtime_test_session_rejects_unknown_family() {
    let mut s = DaemonTestSupervisor::new(Default::default());
    let result = s.start(
        1,
        DaemonTestSelftestRequest {
            executable: "/tmp/oe-selftest".into(),
            family: "unknown".into(),
            selector: None,
            parallelism: 1,
            verbose: false,
            skip_network: false,
        },
        "/tmp/build".into(),
        Vec::new(),
    );
    assert!(result.is_err());
}
