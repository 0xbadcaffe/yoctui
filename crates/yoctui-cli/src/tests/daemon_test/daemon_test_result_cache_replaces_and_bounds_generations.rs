use super::*;

#[test]
fn daemon_test_result_cache_replaces_and_bounds_generations() {
    let mut cache = DaemonTestResultCache::default();
    for generation in 1..=10 {
        cache.insert(DaemonTestResultSnapshot {
            generation,
            records: Vec::new(),
            limitations: Vec::new(),
            complete: true,
        });
    }
    assert!(cache.get(1).is_none());
    assert!(cache.get(10).is_some());
}
