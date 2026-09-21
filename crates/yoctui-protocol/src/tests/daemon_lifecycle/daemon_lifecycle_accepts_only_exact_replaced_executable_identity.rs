use super::*;

#[test]
fn daemon_lifecycle_accepts_only_exact_replaced_executable_identity() {
    let recorded = Path::new("/opt/yoctui/bin/yoctui");
    assert!(executable_matches_record(recorded, recorded));
    assert!(executable_matches_record(
        Path::new("/opt/yoctui/bin/yoctui (deleted)"),
        recorded
    ));
    assert!(!executable_matches_record(
        Path::new("/tmp/yoctui (deleted)"),
        recorded
    ));
    assert!(!executable_matches_record(
        Path::new("/opt/yoctui/bin/yoctui (deleted) (deleted)"),
        recorded
    ));
}
