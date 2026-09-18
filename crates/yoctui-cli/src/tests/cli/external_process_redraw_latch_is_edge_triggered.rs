use super::*;

#[test]
fn external_process_redraw_latch_is_edge_triggered() {
    let latch = RedrawLatch::default();

    assert!(!latch.take());
    latch.request();
    assert!(latch.take());
    assert!(!latch.take());
}
