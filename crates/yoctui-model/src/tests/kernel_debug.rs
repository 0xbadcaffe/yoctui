use super::*;

fn tools() -> KernelDebugTools {
    KernelDebugTools {
        cwd: "/work".into(),
        programs: [
            "ssh",
            "gdb",
            "strace",
            "perf",
            "trace-cmd",
            "cat",
            "dmesg",
            "bpftrace",
            "lttng",
            "crash",
        ]
        .into_iter()
        .map(|name| (name.into(), PathBuf::from(format!("/tools/{name}"))))
        .collect(),
    }
}

#[test]
fn kernel_debug_all_launchable_tools_have_fixed_plans_and_guides_are_non_executable() {
    for tool in KernelDebugTool::ALL {
        assert!(!tool.guide().is_empty());
        assert!(tool.reference().starts_with("https://"));
        let mut draft = KernelDebugDraft::new(tool);
        draft.host = "board.local".into();
        draft.pid = "123".into();
        draft.symbols = "/work/vmlinux symbols".into();
        draft.data = "/work/core".into();
        let result = draft.plan(&tools());
        assert_eq!(
            result.is_ok(),
            tool.program().is_some(),
            "{tool:?}: {result:?}"
        );
        if let Ok(request) = result {
            assert!(request.program.is_absolute());
            assert_eq!(request.kind, TerminalCreationKind::Utility);
            assert!(request.name.contains(if tool.runtime_target() {
                "SSH TARGET"
            } else {
                "HOST, NOT TARGET"
            }));
        }
    }
}

#[test]
fn kernel_debug_gdb_disables_startup_scripts_and_remote_local_inferiors() {
    let mut draft = KernelDebugDraft::new(KernelDebugTool::GdbRemote);
    draft.symbols = "/work/vmlinux".into();
    let request = draft.plan(&tools()).unwrap();
    assert_eq!(
        request.arguments,
        vec![
            "-nx",
            "-nh",
            "-q",
            "-iex",
            "set auto-load off",
            "-iex",
            "set auto-connect-native-target off",
            "--symbols=/work/vmlinux",
            "-ex",
            "target remote 127.0.0.1:1234"
        ]
    );
    draft.endpoint = "127.0.0.1:1234; shell touch /tmp/injection".into();
    assert!(draft.plan(&tools()).is_err());
    draft.endpoint = "|sh".into();
    assert!(draft.plan(&tools()).is_err());
}

#[test]
fn kernel_debug_target_defaults_and_validation_prevent_host_and_command_confusion() {
    let mut draft = KernelDebugDraft::new(KernelDebugTool::Strace);
    assert!(draft.ssh);
    draft.pid = "123".into();
    assert!(draft.plan(&tools()).is_err()); // no implicit host fallback
    draft.host = "board".into();
    let request = draft.plan(&tools()).unwrap();
    assert_eq!(
        request.arguments.last().unwrap(),
        "exec 'strace' '-f' '-tt' '-T' '-p' '123'"
    );
    for value in ["-oProxyCommand=sh", "board;reboot", "user@board", "board\n"] {
        draft.host = value.into();
        assert!(draft.plan(&tools()).is_err());
    }
    draft.host = "board".into();
    for value in ["0", "-1", "2147483648", "1; reboot"] {
        draft.pid = value.into();
        assert!(draft.plan(&tools()).is_err());
    }
    draft.pid = "123".into();
    draft.ssh = false;
    assert_eq!(
        draft.plan(&tools()).unwrap().program,
        PathBuf::from("/tools/strace")
    );
    let mut missing = tools();
    missing.programs.remove("strace");
    assert!(draft.plan(&missing).unwrap_err().contains("missing"));
}

#[test]
fn kernel_debug_bpf_and_file_fields_are_bounded_not_shell_text() {
    let mut draft = KernelDebugDraft::new(KernelDebugTool::Bpftrace);
    draft.host = "board".into();
    let request = draft.plan(&tools()).unwrap();
    assert!(
        request
            .arguments
            .last()
            .unwrap()
            .contains("'tracepoint:syscalls:sys_enter_openat { @[comm] = count(); }'")
    );
    for event in [
        "syscalls:sys_enter_openat { system(\"reboot\"); }",
        "kprobe:do_exit",
        "syscalls:sys_enter_*",
    ] {
        draft.event = event.into();
        assert!(draft.plan(&tools()).is_err());
    }
    draft = KernelDebugDraft::new(KernelDebugTool::GdbCore);
    draft.symbols = "/work/../escape".into();
    draft.data = "/work/core".into();
    assert!(draft.plan(&tools()).is_err());
    draft.symbols = format!("/{}", "x".repeat(4096));
    assert!(draft.plan(&tools()).is_err());
}
