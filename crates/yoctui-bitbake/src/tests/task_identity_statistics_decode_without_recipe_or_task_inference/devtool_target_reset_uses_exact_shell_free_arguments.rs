use super::*;

#[test]
fn devtool_target_reset_uses_exact_shell_free_arguments() {
    let command = authorized_devtool_command(
        "devtool".into(),
        &DevtoolOperation::Reset {
            recipe: "busybox".into(),
        },
    )
    .unwrap();
    assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
    assert_eq!(
        command.arguments(),
        [OsString::from("reset"), OsString::from("busybox")]
    );
}
