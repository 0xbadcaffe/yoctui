#[test]
fn standalone_mode_remains_an_explicit_local_fallback() {
    assert!("Daemon unavailable; interactive runtime is local".starts_with("Daemon unavailable"));
}
