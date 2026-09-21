use super::*;

#[test]
fn compatibility_cache_generation_overflow_fails_closed() {
    let key = material("poky-one", "daemon-one")
        .key(environment("/poky/build", "2.8.1"))
        .unwrap();
    let mut cache = CapabilitySnapshotCache {
        generation: u64::MAX,
        active_key: Some(key),
        snapshot: None,
    };
    assert_eq!(
        cache.invalidate(),
        Err(CapabilityCacheError::GenerationExhausted)
    );
}
