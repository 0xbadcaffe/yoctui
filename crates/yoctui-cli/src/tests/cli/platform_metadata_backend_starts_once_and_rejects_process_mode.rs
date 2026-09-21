use super::*;

#[test]
fn platform_metadata_backend_starts_once_and_rejects_process_mode() {
    assert_eq!(
        metadata_backend_start_required(&Backend::Bridge, false),
        Ok(true)
    );
    assert_eq!(
        metadata_backend_start_required(&Backend::Bridge, true),
        Ok(false)
    );
    assert!(
        metadata_backend_start_required(&Backend::Process, false)
            .expect_err("process mode must reject authoritative metadata inspection")
            .contains("require --backend bridge")
    );
}
