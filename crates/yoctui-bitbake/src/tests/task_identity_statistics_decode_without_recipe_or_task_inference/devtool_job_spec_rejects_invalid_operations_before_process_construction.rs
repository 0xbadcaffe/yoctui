use super::*;

#[test]
fn devtool_job_spec_rejects_invalid_operations_before_process_construction() {
    assert!(matches!(
        authorized_devtool_command("devtool".into(), &DevtoolOperation::Reset {
            recipe: "--help".into(),
        }),
        Err(DevtoolCompatibilityError::InvalidRequest(message)) if message.contains("recipe")
    ));
    assert!(matches!(
        authorized_devtool_command("devtool".into(), &DevtoolOperation::DeployTarget {
            recipe: "busybox".into(),
            target: "root@host\n--help".into(),
        }),
        Err(DevtoolCompatibilityError::InvalidRequest(message)) if message.contains("target")
    ));
    assert!(matches!(
        authorized_devtool_command("devtool".into(), &DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: "meta-custom".into(),
        }),
        Err(DevtoolCompatibilityError::InvalidRequest(message)) if message.contains("absolute")
    ));
}
