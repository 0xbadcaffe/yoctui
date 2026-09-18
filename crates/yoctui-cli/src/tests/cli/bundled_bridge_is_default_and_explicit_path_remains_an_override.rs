use super::*;

#[test]
fn bundled_bridge_is_default_and_explicit_path_remains_an_override() {
    assert_eq!(bridge_path_override(None), None);
    assert_eq!(
        bridge_path_override(Some("/opt/yoctui/bridge.py".into())),
        Some(PathBuf::from("/opt/yoctui/bridge.py"))
    );
}
