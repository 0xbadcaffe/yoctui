use super::*;

#[test]
fn client_runtime_security_rejects_invalid_generation() {
    assert!(
        DaemonSecuritySupervisor::new(Default::default())
            .start(0, vec!["/tmp/report.json".into()])
            .is_err()
    );
}
