use super::*;

#[test]
fn normal_render_interval_is_capped_at_ten_hertz() {
    assert_eq!(
        interactive_frame_interval(Duration::from_millis(16)),
        Duration::from_millis(100)
    );
    assert_eq!(
        interactive_frame_interval(Duration::from_millis(250)),
        Duration::from_millis(250)
    );
}
