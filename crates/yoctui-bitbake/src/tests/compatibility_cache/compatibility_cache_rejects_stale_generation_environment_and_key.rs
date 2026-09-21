use super::*;

#[test]
fn compatibility_cache_rejects_stale_generation_environment_and_key() {
    let base_environment = environment("/poky/build", "2.8.1");
    let key = material("poky-one", "daemon-one")
        .key(base_environment.clone())
        .unwrap();
    let other = material("poky-two", "daemon-two")
        .key(environment("/other/build", "2.8.1"))
        .unwrap();
    let mut cache = CapabilitySnapshotCache::default();
    let generation = cache.select(key.clone()).unwrap().generation;
    assert_eq!(
        cache.store(
            &key,
            generation + 1,
            snapshot(generation, base_environment.clone()),
        ),
        Err(CapabilityCacheError::StaleOrMismatched)
    );
    assert_eq!(
        cache.store(&other, generation, snapshot(generation, base_environment)),
        Err(CapabilityCacheError::StaleOrMismatched)
    );
    assert_eq!(cache.invalidate().unwrap(), 2);
    assert!(cache.lookup(&key).is_none());
}
