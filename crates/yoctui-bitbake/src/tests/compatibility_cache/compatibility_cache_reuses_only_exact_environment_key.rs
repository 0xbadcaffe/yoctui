use super::*;

#[test]
fn compatibility_cache_reuses_only_exact_environment_key() {
    let env = environment("/poky/build", "2.8.1");
    let key = material("poky-one", "daemon-one").key(env.clone()).unwrap();
    let mut cache = CapabilitySnapshotCache::default();
    let first = cache.select(key.clone()).unwrap();
    assert_eq!(first.generation, 1);
    assert!(first.snapshot.is_none());
    cache
        .store(&key, first.generation, snapshot(first.generation, env))
        .unwrap();
    let reused = cache.select(key.clone()).unwrap();
    assert_eq!(reused.generation, 1);
    assert!(reused.snapshot.is_some());
    assert!(cache.lookup(&key).is_some());
}
