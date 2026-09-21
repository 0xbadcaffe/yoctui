use super::*;

#[test]
fn client_runtime_maintenance_rejects_invalid_request() {
    assert!(inspect(0, "/build".into(), None, None, Vec::new(), Vec::new()).is_err());
}
