use super::*;

#[test]
fn client_runtime_qa_task_runner_rejects_invalid_request() {
    let mut supervisor = DaemonQaSupervisor::new(Default::default());
    let result = supervisor.start(
        0,
        0,
        "invalid".into(),
        "layer".into(),
        "relative".into(),
        "/missing/yocto-check-layer".into(),
        vec!["relative".into()],
        Vec::new(),
    );
    assert!(result.is_err());
}
