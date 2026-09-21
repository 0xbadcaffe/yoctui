use super::*;

#[cfg(unix)]
#[test]
fn devtool_job_spec_preserves_non_utf8_finish_destination() {
    use std::os::unix::ffi::OsStringExt;

    let mut bytes = b"/layers/meta-".to_vec();
    bytes.push(0xff);
    let destination = PathBuf::from(OsString::from_vec(bytes.clone()));
    let command = authorized_devtool_command(
        "devtool".into(),
        &DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination,
        },
    )
    .unwrap();
    assert_eq!(command.arguments()[2], OsString::from_vec(bytes));
}
