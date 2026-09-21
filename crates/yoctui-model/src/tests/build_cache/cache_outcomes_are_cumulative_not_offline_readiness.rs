use super::*;

#[test]
fn cache_outcomes_are_cumulative_not_offline_readiness() {
    let mut cache = BuildCacheState::default();
    for _ in 0..5000 {
        cache.record_outcome("do_fetch", true);
    }
    cache.record_outcome("do_fetch", false);
    cache.record_outcome("do_compile_setscene", true);
    cache.record_outcome("do_compile", false);
    assert_eq!(cache.fetch_completed, 5000);
    assert_eq!(cache.fetch_failed, 1);
    assert_eq!(cache.setscene_completed, 1);
    assert_eq!(cache.setscene_failed, 0);
    assert_eq!(cache.summary, None);
}
