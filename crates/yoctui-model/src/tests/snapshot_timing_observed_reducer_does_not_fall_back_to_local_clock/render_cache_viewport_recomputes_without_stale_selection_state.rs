use super::*;

#[test]
fn render_cache_viewport_recomputes_without_stale_selection_state() {
    assert_eq!(centered_viewport_range(None, 100, 10), 0..10);
    assert_eq!(centered_viewport_range(Some(50), 100, 10), 45..55);
    assert_eq!(centered_viewport_range(Some(99), 100, 10), 90..100);
    assert_eq!(
        centered_viewport_range(Some(99), 3, 10),
        0..3,
        "query/inventory shrink clamps a formerly valid selection immediately"
    );
    assert_eq!(centered_viewport_range(Some(1), 0, 10), 0..0);
    assert_eq!(centered_viewport_range(Some(1), 10, 0), 0..0);
}
