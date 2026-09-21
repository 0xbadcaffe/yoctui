use super::*;

#[test]
fn client_runtime_maintenance_service_rejects_invalid_request() {
    assert!(
        inspect_services(
            0,
            "/build".into(),
            None,
            None,
            None,
            None,
            Vec::new(),
            "/proc".into()
        )
        .is_err()
    );
}
