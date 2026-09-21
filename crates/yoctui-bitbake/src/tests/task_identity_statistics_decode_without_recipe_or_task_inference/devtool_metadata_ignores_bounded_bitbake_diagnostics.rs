use super::*;

#[test]
fn devtool_metadata_ignores_bounded_bitbake_diagnostics() {
    assert_eq!(
        parse_devtool_status(
            "NOTE: Starting bitbake server...\nWARNING: using existing server\nbusybox: /workspace/sources/busybox (/layers/core/busybox_1.0.bb)\n"
        ),
        Ok(vec![(
            "busybox".into(),
            "/workspace/sources/busybox".into(),
            Some("/layers/core/busybox_1.0.bb".into()),
        )])
    );
}
