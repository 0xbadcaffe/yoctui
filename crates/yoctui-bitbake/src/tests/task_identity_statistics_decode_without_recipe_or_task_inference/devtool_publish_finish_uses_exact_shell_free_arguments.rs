use super::*;

#[test]
fn devtool_publish_finish_uses_exact_shell_free_arguments() {
    let command = authorized_devtool_command(
        "devtool".into(),
        &DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: "/layers/meta-demo".into(),
        },
    )
    .unwrap();
    assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
    assert_eq!(
        command.arguments(),
        [
            OsString::from("finish"),
            OsString::from("busybox"),
            OsString::from("/layers/meta-demo"),
        ]
    );
}
