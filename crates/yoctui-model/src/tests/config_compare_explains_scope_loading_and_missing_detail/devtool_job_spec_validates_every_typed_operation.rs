use super::*;

#[test]
fn devtool_job_spec_validates_every_typed_operation() {
    let operations = [
        DevtoolOperation::Modify {
            recipe: "busybox".into(),
        },
        DevtoolOperation::UpdateRecipe {
            recipe: "busybox".into(),
        },
        DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: "/layers/meta-custom".into(),
        },
        DevtoolOperation::DeployTarget {
            recipe: "busybox".into(),
            target: "root@192.0.2.1:/opt".into(),
        },
        DevtoolOperation::UndeployTarget {
            recipe: "busybox".into(),
            target: "root@192.0.2.1".into(),
        },
        DevtoolOperation::Reset {
            recipe: "busybox".into(),
        },
    ];
    for operation in operations {
        assert_eq!(operation.recipe(), "busybox");
        assert_eq!(operation.validate(), Ok(()));
    }
}
