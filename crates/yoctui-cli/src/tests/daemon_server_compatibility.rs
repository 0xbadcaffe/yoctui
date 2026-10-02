use super::*;

#[test]
fn waiting_backend_diagnostic_distinguishes_scheduled_and_suppressed_recovery() {
    assert_eq!(
        waiting_backend_diagnostic(false),
        "Waiting for BitBake API discovery; retrying the backend probe"
    );
    assert_eq!(
        waiting_backend_diagnostic(true),
        "Waiting for BitBake API discovery; automatic recovery is disabled for an explicit bridge override"
    );
    assert!(!waiting_backend_diagnostic(true).contains("retrying"));
}
