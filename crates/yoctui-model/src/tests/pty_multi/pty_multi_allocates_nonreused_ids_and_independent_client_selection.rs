use super::*;

#[test]
fn pty_multi_allocates_nonreused_ids_and_independent_client_selection() {
    let mut registry = PtySessionRegistry::new(3, 3, 2).unwrap();
    let first = registry.reserve_id().unwrap();
    let second = registry.reserve_id().unwrap();
    registry.insert(session(first, "build", 10)).unwrap();
    registry.insert(session(second, "menuconfig", 11)).unwrap();
    let client_a = PtyClientId([1; 16]);
    let client_b = PtyClientId([2; 16]);
    registry.switch(client_a, first).unwrap();
    registry.switch(client_b, second).unwrap();
    assert_eq!(registry.selected(client_a), Some(first));
    assert_eq!(registry.selected(client_b), Some(second));
    registry.rename(second, "kernel config".into()).unwrap();
    assert_eq!(registry.get(second).unwrap().name, "kernel config");
    assert_eq!(
        registry.rename(second, "build".into()),
        Err(PtyRegistryError::DuplicateName("build".into()))
    );
    let third = registry.reserve_id().unwrap();
    assert!(third.0 > second.0);
    registry.cancel_reservation(third).unwrap();
    let fourth = registry.reserve_id().unwrap();
    assert!(fourth.0 > third.0);
}
