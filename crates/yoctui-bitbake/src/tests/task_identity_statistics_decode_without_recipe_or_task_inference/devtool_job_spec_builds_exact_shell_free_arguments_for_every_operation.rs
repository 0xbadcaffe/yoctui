use super::*;

#[test]
fn devtool_job_spec_builds_exact_shell_free_arguments_for_every_operation() {
    let cases = [
        (
            DevtoolOperation::Modify {
                recipe: "busybox".into(),
            },
            vec!["modify", "busybox"],
        ),
        (
            DevtoolOperation::UpdateRecipe {
                recipe: "busybox".into(),
            },
            vec!["update-recipe", "busybox"],
        ),
        (
            DevtoolOperation::Finish {
                recipe: "busybox".into(),
                destination: "/layers/meta-custom".into(),
            },
            vec!["finish", "busybox", "/layers/meta-custom"],
        ),
        (
            DevtoolOperation::DeployTarget {
                recipe: "busybox".into(),
                target: "root@192.0.2.1:/opt".into(),
            },
            vec!["deploy-target", "busybox", "root@192.0.2.1:/opt"],
        ),
        (
            DevtoolOperation::UndeployTarget {
                recipe: "busybox".into(),
                target: "root@192.0.2.1".into(),
            },
            vec!["undeploy-target", "busybox", "root@192.0.2.1"],
        ),
        (
            DevtoolOperation::Reset {
                recipe: "busybox".into(),
            },
            vec!["reset", "busybox"],
        ),
    ];
    for (operation, expected) in cases {
        let command = authorized_devtool_command("devtool".into(), &operation).unwrap();
        assert_eq!(command.executable(), Path::new("/test/bin/devtool"));
        assert_eq!(
            command.arguments(),
            expected.into_iter().map(OsString::from).collect::<Vec<_>>()
        );
    }
}
