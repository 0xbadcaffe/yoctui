use super::*;

#[test]
fn log_filters_combine_severity_recipe_task_and_search() {
    let mut logs = LogState::new(10, 1_000);
    logs.insert(tagged_log(
        "busybox",
        "do_compile",
        Severity::Warning,
        "Compiler warning",
    ));
    logs.insert(tagged_log(
        "bash",
        "do_install",
        Severity::Warning,
        "Install warning",
    ));
    logs.filter = Some(Severity::Warning);
    logs.recipe_filter = Some("busybox".into());
    logs.task_filter = Some("do_compile".into());
    logs.query = "compiler".into();
    assert_eq!(logs.filtered().count(), 1);
}
