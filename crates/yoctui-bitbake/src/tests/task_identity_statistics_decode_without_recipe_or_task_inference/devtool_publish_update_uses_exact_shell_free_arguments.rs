use super::*;

#[test]
fn devtool_publish_update_uses_exact_shell_free_arguments() {
    let command = authorized_devtool_command(
        "devtool".into(),
        &DevtoolOperation::UpdateRecipe {
            recipe: "busybox".into(),
        },
    )
    .unwrap();
    assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
    assert_eq!(
        command.arguments(),
        [OsString::from("update-recipe"), OsString::from("busybox")]
    );
}
