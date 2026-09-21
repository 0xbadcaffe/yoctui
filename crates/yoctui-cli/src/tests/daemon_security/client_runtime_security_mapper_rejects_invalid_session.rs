use super::*;

#[test]
fn client_runtime_security_mapper_rejects_invalid_session() {
    assert!(
        DaemonSecurityMapperSupervisor::new(Default::default())
            .start(
                0,
                "/missing/cve-check-map-pkgs".into(),
                vec!["/tmp/report".into()],
                vec!["/tmp/report".into()],
            )
            .is_err()
    );
}
