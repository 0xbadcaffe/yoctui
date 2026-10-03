use super::*;
use crate::{
    Action, App, Dialog, Effect, FocusTarget, KernelDebugAction as A, KernelDebugOperation as Op,
    KernelDebugResult as R, PlatformView, Screen, update,
};

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
            "runqemu",
            "yoctui",
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
        draft.serial = KgdbSerialDraft {
            config: "/work/.config".into(),
            device: "/dev/ttyUSB0".into(),
            target_uart: "ttyS0".into(),
            ready: "yes".into(),
            ..Default::default()
        };
        draft.qemu = crate::QemuDebugDraft {
            build_dir: "/work".into(),
            qemuboot: "/work/image.qemuboot.conf".into(),
            kernel: "/work/bzImage".into(),
            rootfs: "/work/image.ext4".into(),
            ..crate::QemuDebugDraft::default()
        };
        let result = draft.plan(&tools());
        assert_eq!(
            result.is_ok(),
            tool.program().is_some(),
            "{tool:?}: {result:?}"
        );
        if let Ok(request) = result {
            assert!(request.program.is_absolute());
            assert_eq!(request.kind, TerminalCreationKind::Utility);
            assert!(
                request
                    .name
                    .contains(if tool == KernelDebugTool::KgdbSerial {
                        "BOARD TARGET"
                    } else if tool.runtime_target() {
                        "SSH TARGET"
                    } else {
                        "HOST, NOT TARGET"
                    })
            );
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
            "set debuginfod enabled off",
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

fn serial_app() -> App {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    app.kernel_debug.tools = Some(tools());
    app.kernel_debug.selection = 17;
    update(&mut app, Action::KernelDebug(A::OpenSelected));
    let Some(Dialog::KernelDebug(d)) = app.active_dialog_mut() else {
        panic!()
    };
    d.draft.symbols = "/work/vmlinux".into();
    d.draft.serial = KgdbSerialDraft {
        config: "/work/.config".into(),
        device: "/dev/ttyUSB0".into(),
        target_uart: "ttyAMA0".into(),
        ready: "yes".into(),
        ..Default::default()
    };
    app
}

fn serial_result(request: &crate::KernelDebugRequest) -> R {
    let Op::Prepare { draft, tools } = &request.operation else {
        panic!()
    };
    R::PreparedSerial {
        request: draft.plan(tools).unwrap(),
        report: crate::KgdbConfigReport::inspect(
            "CONFIG_KGDB=y\nCONFIG_KGDB_SERIAL_CONSOLE=y\nCONFIG_DEBUG_INFO=y\n",
        )
        .unwrap(),
    }
}

#[test]
fn kernel_debug_serial_requires_six_explicit_fields_and_readiness_before_review() {
    let mut app = serial_app();
    let Some(Dialog::KernelDebug(d)) = app.active_dialog_mut() else {
        panic!()
    };
    assert_eq!(
        d.draft.fields(),
        vec![
            KernelDebugField::Symbols,
            KernelDebugField::KernelConfig,
            KernelDebugField::SerialDevice,
            KernelDebugField::SerialBaud,
            KernelDebugField::TargetUart,
            KernelDebugField::Ready
        ]
    );
    d.draft.serial.ready.clear();
    assert!(update(&mut app, Action::KernelDebug(A::Review)).is_none());
    assert!(
        matches!(app.active_dialog(), Some(Dialog::KernelDebug(d)) if d.error.as_deref().unwrap().contains("readiness"))
    );
    assert!(app.kernel_debug.pending.is_none());
    update(&mut app, Action::KernelDebug(A::Field(5)));
    update(&mut app, Action::KernelDebug(A::Insert("yes".into())));
    assert!(update(&mut app, Action::KernelDebug(A::Review)).is_some());
    assert!(app.daemon.pty_sessions.is_empty());
}

#[test]
fn kernel_debug_serial_typed_preview_launch_cancel_and_detached_use_existing_lifecycle() {
    let mut app = serial_app();
    let Some(Effect::KernelDebug(request)) = update(&mut app, Action::KernelDebug(A::Review))
    else {
        panic!()
    };
    update(
        &mut app,
        Action::KernelDebug(A::Finished {
            generation: request.generation,
            result: Ok(serial_result(&request)),
        }),
    );
    let preview = app.kernel_debug.serial_preview.as_ref().unwrap();
    assert_eq!(preview.spec.target_uart, "ttyAMA0");
    assert_eq!(preview.report.options["CONFIG_KGDB"].as_deref(), Some("y"));
    assert!(app.kernel_debug.qemu_preview.is_none());
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(_))
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);
    let expected = app.kernel_debug.prepared.clone().unwrap();
    let mut cancelled = app.clone();
    assert!(update(&mut cancelled, Action::CancelTerminalLaunch).is_none());
    assert!(cancelled.active_dialog().is_none());
    assert!(cancelled.daemon.pty_sessions.is_empty());
    let mut detached = app.clone();
    detached.detached_terminal = crate::DetachedTerminalAvailability::Available {
        launcher: "test".into(),
    };
    update(
        &mut detached,
        Action::SelectTerminalLaunchDestination { delta: 1 },
    );
    assert!(
        matches!(update(&mut detached, Action::ConfirmTerminalLaunch), Some(Effect::LaunchDetachedTerminal(request)) if request == expected)
    );
    assert!(
        matches!(update(&mut app, Action::ConfirmTerminalLaunch), Some(Effect::Terminal(crate::TerminalEffect::Create { program, arguments, .. })) if program == expected.program && arguments == expected.arguments)
    );
    assert_eq!(app.screen, Screen::TerminalSessions);
    assert_eq!(app.focus, FocusTarget::Workspace);
}

#[test]
fn kernel_debug_serial_cancel_stale_covered_changed_and_wrong_results_cannot_launch() {
    for failure in ["cancel", "covered", "changed", "wrong", "error", "stale"] {
        let mut app = serial_app();
        let Some(Effect::KernelDebug(request)) = update(&mut app, Action::KernelDebug(A::Review))
        else {
            panic!()
        };
        let mut result = Ok(serial_result(&request));
        match failure {
            "cancel" => {
                update(&mut app, Action::KernelDebug(A::Cancel));
            }
            "covered" => app.command_palette_open = true,
            "changed" => {
                let Some(Dialog::KernelDebug(d)) = app.active_dialog_mut() else {
                    panic!()
                };
                d.draft.serial.baud = "9600".into();
            }
            "wrong" => {
                let R::PreparedSerial { request, .. } = result.unwrap() else {
                    panic!()
                };
                result = Ok(R::Prepared(request));
            }
            "error" => result = Err("serial config unavailable".into()),
            "stale" => app.kernel_debug.generation += 1,
            _ => unreachable!(),
        }
        update(
            &mut app,
            Action::KernelDebug(A::Finished {
                generation: request.generation,
                result,
            }),
        );
        assert!(app.kernel_debug.serial_preview.is_none(), "{failure}");
        assert!(
            !matches!(app.active_dialog(), Some(Dialog::TerminalLaunch(_))),
            "{failure}"
        );
        assert!(app.daemon.pty_sessions.is_empty());
        if failure == "error" {
            assert!(
                matches!(app.active_dialog(), Some(Dialog::KernelDebug(d)) if d.error.as_deref() == Some("serial config unavailable"))
            );
        }
    }
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

#[test]
fn kernel_debug_reducer_preview_is_correlated_and_cancel_drops_late_preparation() {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    let Some(Effect::KernelDebug(request)) = update(&mut app, Action::KernelDebug(A::Open)) else {
        panic!("discovery requested")
    };
    update(
        &mut app,
        Action::KernelDebug(A::Finished {
            generation: request.generation,
            result: Ok(R::Tools(tools())),
        }),
    );
    assert!(app.kernel_debug.visible);
    update(&mut app, Action::KernelDebug(A::SelectAt(2)));
    update(&mut app, Action::KernelDebug(A::OpenSelected));
    update(&mut app, Action::Open(Screen::Dashboard));
    assert_eq!(app.screen, Screen::Kernel); // modal focus trap
    update(&mut app, Action::KernelDebug(A::Field(1)));
    update(&mut app, Action::KernelDebug(A::Insert("board".into())));
    update(&mut app, Action::KernelDebug(A::Field(3)));
    update(&mut app, Action::KernelDebug(A::Insert("123".into())));
    let Some(Effect::KernelDebug(request)) = update(&mut app, Action::KernelDebug(A::Review))
    else {
        panic!("preparation requested")
    };
    let Op::Prepare { draft, tools } = &request.operation else {
        panic!()
    };
    let prepared = draft.plan(tools).unwrap();
    update(&mut app, Action::KernelDebug(A::Cancel));
    update(
        &mut app,
        Action::KernelDebug(A::Finished {
            generation: request.generation,
            result: Ok(R::Prepared(prepared)),
        }),
    );
    assert!(app.active_dialog().is_none());
    assert!(app.kernel_debug.pending.is_none());
    assert!(app.kernel_debug.prepared.is_none());
}

#[test]
fn kernel_debug_reducer_preparation_failure_preserves_draft_and_success_only_opens_preview() {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    app.kernel_debug.tools = Some(tools());
    app.kernel_debug.selection = 6; // dmesg
    update(&mut app, Action::KernelDebug(A::OpenSelected));
    update(&mut app, Action::KernelDebug(A::ChangeScope)); // explicit host
    let Some(Effect::KernelDebug(request)) = update(&mut app, Action::KernelDebug(A::Review))
    else {
        panic!()
    };
    update(
        &mut app,
        Action::KernelDebug(A::Finished {
            generation: request.generation,
            result: Err("missing executable".into()),
        }),
    );
    assert!(
        matches!(app.active_dialog(), Some(Dialog::KernelDebug(dialog)) if !dialog.draft.ssh && dialog.error.as_deref() == Some("missing executable"))
    );
    let Some(Effect::KernelDebug(request)) = update(&mut app, Action::KernelDebug(A::Review))
    else {
        panic!()
    };
    let Op::Prepare { draft, tools } = request.operation else {
        panic!()
    };
    update(
        &mut app,
        Action::KernelDebug(A::Finished {
            generation: request.generation,
            result: Ok(R::Prepared(draft.plan(&tools).unwrap())),
        }),
    );
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(_))
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert!(app.daemon.pty_sessions.is_empty());
    let expected = app.kernel_debug.prepared.clone().unwrap();
    assert!(
        matches!(update(&mut app, Action::ConfirmTerminalLaunch), Some(Effect::Terminal(crate::TerminalEffect::Create { program, arguments, .. })) if program == expected.program && arguments == expected.arguments)
    );
    assert_eq!(app.screen, Screen::TerminalSessions);
    assert_eq!(app.focus, FocusTarget::Workspace);
}

#[test]
fn kernel_debug_three_kernel_views_preserve_two_firmware_views_and_guides_never_launch() {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    update(&mut app, Action::CycleKernelView);
    assert_eq!(app.kernel.view, PlatformView::DeviceTrees);
    update(&mut app, Action::CycleKernelView);
    assert!(app.kernel_debug.visible);
    update(&mut app, Action::CycleKernelView);
    assert!(!app.kernel_debug.visible);
    assert_eq!(app.kernel.view, PlatformView::Configuration);
    update(&mut app, Action::CycleFirmwareView);
    update(&mut app, Action::CycleFirmwareView);
    assert_eq!(app.firmware.view, PlatformView::Configuration);
    app.kernel_debug.selection = 15;
    app.kernel_debug.pending = None;
    update(&mut app, Action::KernelDebug(A::OpenSelected));
    assert_eq!(update(&mut app, Action::KernelDebug(A::Review)), None);
    assert!(
        matches!(app.active_dialog(), Some(Dialog::KernelDebug(dialog)) if dialog.draft.tool == KernelDebugTool::SysrqKdump)
    );
}

include!("kernel_debug/managed_qemu.rs");
