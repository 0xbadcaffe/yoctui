use super::*;

#[test]
fn pty_multi_enforces_session_and_client_limits() {
    let mut registry = PtySessionRegistry::new(1, 1, 1).unwrap();
    let id = registry.reserve_id().unwrap();
    assert_eq!(
        registry.reserve_id(),
        Err(PtyRegistryError::SessionLimit(1))
    );
    registry.insert(session(id, "only", 10)).unwrap();
    registry.switch(PtyClientId([1; 16]), id).unwrap();
    assert_eq!(
        registry.switch(PtyClientId([2; 16]), id),
        Err(PtyRegistryError::ClientLimit(1))
    );
}
