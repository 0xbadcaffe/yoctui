use super::*;

#[test]
fn client_runtime_maintenance_sstate_rejects_invalid_session() {
    assert!(
        DaemonMaintenanceSupervisor::new(Default::default())
            .start_readiness(
                0,
                1,
                1,
                "/build".into(),
                None,
                None,
                Vec::new(),
                Vec::new(),
                vec!["core-image-minimal".into()],
                "isolated_tmpdir".into(),
                None,
                None,
                1,
            )
            .is_err()
    );
}
