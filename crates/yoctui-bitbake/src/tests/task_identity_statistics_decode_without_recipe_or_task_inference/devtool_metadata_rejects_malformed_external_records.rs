use super::*;

#[test]
fn devtool_metadata_rejects_malformed_external_records() {
    assert_eq!(
        parse_devtool_status("busybox relative/path"),
        Err("busybox relative/path".into())
    );
    assert_eq!(
        parse_git_status("unexpected", Some("/workspace/busybox".into())),
        Err("unrecognized Git status record: unexpected".into())
    );
}
