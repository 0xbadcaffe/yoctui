use super::*;

#[cfg(unix)]
#[test]
fn devtool_publish_finish_preserves_native_destination_bytes() {
    use std::os::unix::ffi::OsStringExt;

    let mut bytes = b"/layers/meta-".to_vec();
    bytes.push(0xfe);
    let command = authorized_devtool_command(
        "devtool".into(),
        &DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: PathBuf::from(OsString::from_vec(bytes.clone())),
        },
    )
    .unwrap();
    assert_eq!(command.arguments()[2], OsString::from_vec(bytes));
}
