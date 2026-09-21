use super::*;

#[test]
fn client_runtime_maintenance_release_rejects_invalid_session() {
    assert!(
        DaemonMaintenanceSupervisor::new(Default::default())
            .start_external(
                0,
                "/missing/tool".into(),
                "tool".into(),
                vec!["--help".into()],
                "/build".into()
            )
            .is_err()
    );
}
