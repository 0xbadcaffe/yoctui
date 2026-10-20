use super::*;

#[test]
fn perf_defaults_to_software_sampling_and_validates_each_explicit_counter() {
    let mut draft = KernelDebugDraft::new(KernelDebugTool::Perf);
    draft.ssh = false;
    draft.pid = "123".into();
    assert_eq!(draft.event, "cpu-clock");
    assert!(draft.fields().contains(&KernelDebugField::PerfEvent));
    for counter in ["cpu-clock", "task-clock", "cycles", "instructions"] {
        draft.event = counter.into();
        let request = draft.plan(&tools()).unwrap();
        assert_eq!(request.arguments, ["top", "-e", counter, "-p", "123"]);
    }
    for invalid in [
        "",
        "--help",
        "cpu-clock;reboot",
        "cpu-clock\n",
        "arbitrary:event",
        "cpu-clock -a",
    ] {
        draft.event = invalid.into();
        assert!(draft.plan(&tools()).is_err(), "{invalid:?}");
    }
}

#[test]
fn perf_target_does_not_require_a_host_perf_installation() {
    let mut tools = tools();
    tools.programs.remove("perf");
    let mut draft = KernelDebugDraft::new(KernelDebugTool::Perf);
    draft.host = "127.0.0.1".into();
    draft.port = "2245".into();
    draft.pid = "456".into();
    let request = draft.plan(&tools).unwrap();
    assert_eq!(request.program, PathBuf::from("/tools/ssh"));
    assert_eq!(
        request.arguments.last().unwrap(),
        "exec 'perf' 'top' '-e' 'cpu-clock' '-p' '456'"
    );
    draft.ssh = false;
    assert!(draft.plan(&tools).is_err());
}
