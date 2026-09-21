use super::*;

#[test]
fn devtool_target_deploy_validates_before_exact_shell_free_arguments() {
    let operation = DevtoolOperation::DeployTarget {
        recipe: "busybox".into(),
        target: "root@192.0.2.1:/opt/demo".into(),
    };
    let command = authorized_devtool_command("devtool".into(), &operation).unwrap();
    assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
    assert_eq!(
        command.arguments(),
        [
            OsString::from("deploy-target"),
            OsString::from("busybox"),
            OsString::from("root@192.0.2.1:/opt/demo"),
        ]
    );
    assert!(matches!(
        authorized_devtool_command("devtool".into(), &DevtoolOperation::DeployTarget {
            recipe: "busybox".into(),
            target: "--help".into(),
        }),
        Err(DevtoolCompatibilityError::InvalidRequest(message)) if message.contains("target")
    ));
}
