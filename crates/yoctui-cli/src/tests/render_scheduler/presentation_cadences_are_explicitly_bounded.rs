use super::*;

#[test]
fn presentation_cadences_are_explicitly_bounded() {
    assert!(ANIMATION_INTERVAL >= std::time::Duration::from_millis(100));
    assert!(ANIMATION_INTERVAL <= std::time::Duration::from_millis(250));
    assert_eq!(
        ORDINARY_FRAME_INTERVAL,
        std::time::Duration::from_millis(250)
    );
    assert_eq!(ELAPSED_REFRESH_INTERVAL, std::time::Duration::from_secs(1));
}
