use super::*;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);

#[tokio::test]
async fn kernel_debug_instrumentation_worker_inspects_then_only_exports_after_confirmation() {
    use yoctui_model::{Action, App, Dialog, KernelDebugAction as A, Screen};
    for scenario in [
        "success",
        "cancel",
        "covered",
        "bad-config",
        "changed",
        "collision",
        "export-escape",
    ] {
        let root = std::env::temp_dir().join(format!(
            "yoctui-instrumentation-worker-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let config = root.join(".config");
        let output = root.join("debug.cfg");
        let contents = if scenario == "bad-config" {
            "CONFIG_KASAN=y\nCONFIG_KASAN=n\n"
        } else {
            "CONFIG_DEBUG_KERNEL=y\n"
        };
        fs::write(&config, contents).unwrap();
        let mut app = App::new(32, 4096);
        app.onboarding.open = false;
        app.screen = Screen::Kernel;
        app.kernel_debug.selection = 13;
        yoctui_model::update(&mut app, Action::KernelDebug(A::OpenSelected));
        let Some(Dialog::KernelDebug(d)) = app.active_dialog_mut() else {
            panic!()
        };
        d.draft.instrumentation.config = config.display().to_string();
        d.draft.instrumentation.output = output.display().to_string();
        let mut io = KernelDebugIo::default();
        io.submit(yoctui_model::update(&mut app, Action::KernelDebug(A::Review)).unwrap());
        if scenario == "cancel" {
            yoctui_model::update(&mut app, Action::KernelDebug(A::Cancel));
        }
        if scenario == "covered" {
            app.command_palette_open = true;
        }
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !io.poll(&mut app).await {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(!output.exists());
        assert_eq!(fs::read_to_string(&config).unwrap(), contents);
        if matches!(scenario, "cancel" | "covered" | "bad-config") {
            assert!(
                app.kernel_debug.instrumentation_preview.is_none(),
                "{scenario}"
            );
        } else {
            let preview = app.kernel_debug.instrumentation_preview.clone().unwrap();
            if scenario == "changed" {
                fs::write(&config, "CONFIG_KASAN=y\n").unwrap();
            }
            if scenario == "collision" {
                fs::write(&output, "user-owned file").unwrap();
            }
            io.submit(yoctui_model::update(&mut app, Action::KernelDebug(A::Review)).unwrap());
            if scenario == "export-escape" {
                yoctui_model::update(&mut app, Action::KernelDebug(A::Cancel));
                assert!(app.active_dialog().is_some());
                assert!(app.kernel_debug.pending.is_some());
            }
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                while !io.poll(&mut app).await {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            match scenario {
                "changed" => {
                    assert!(!output.exists());
                    assert!(
                        app.kernel_debug
                            .error
                            .as_deref()
                            .unwrap()
                            .contains("changed since review")
                    );
                }
                "collision" => assert_eq!(fs::read_to_string(&output).unwrap(), "user-owned file"),
                _ => {
                    assert_eq!(
                        fs::read_to_string(&output).unwrap(),
                        preview.draft.preset.fragment()
                    );
                    assert_eq!(fs::read_to_string(&config).unwrap(), contents);
                    assert!(app.active_dialog().is_none());
                    assert!(app.notification.as_deref().unwrap().contains("Not applied"));
                }
            }
        }
        assert!(app.daemon.pty_sessions.is_empty());
        assert!(app.kernel_debug.prepared.is_none());
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn kernel_debug_serial_worker_preserves_typed_report_and_cancel_error_boundaries() {
    use yoctui_model::{Action, App, Dialog, KernelDebugAction as A, Screen};
    for scenario in ["success", "cancel", "error", "covered"] {
        let fixture = crate::kgdb_serial::tests::fixture();
        let mut app = App::new(32, 4096);
        app.onboarding.open = false;
        app.screen = Screen::Kernel;
        app.kernel_debug.tools = Some(KernelDebugTools {
            cwd: fixture.spec.cwd.clone(),
            programs: [
                ("gdb".into(), fixture.spec.gdb.clone()),
                ("yoctui".into(), std::env::current_exe().unwrap()),
            ]
            .into(),
        });
        app.kernel_debug.selection = 17;
        yoctui_model::update(&mut app, Action::KernelDebug(A::OpenSelected));
        let Some(Dialog::KernelDebug(d)) = app.active_dialog_mut() else {
            panic!()
        };
        d.draft.symbols = fixture.spec.symbols.display().to_string();
        d.draft.serial = yoctui_model::KgdbSerialDraft {
            config: fixture.spec.config.display().to_string(),
            device: fixture.spec.device.display().to_string(),
            target_uart: fixture.spec.target_uart.clone(),
            ready: "yes".into(),
            ..Default::default()
        };
        if scenario == "error" {
            fs::write(&fixture.spec.config, "CONFIG_KGDB=n\n").unwrap();
        }
        let effect = yoctui_model::update(&mut app, Action::KernelDebug(A::Review)).unwrap();
        let mut worker = KernelDebugIo::default();
        worker.submit(effect);
        if scenario == "cancel" {
            yoctui_model::update(&mut app, Action::KernelDebug(A::Cancel));
        }
        if scenario == "covered" {
            app.command_palette_open = true;
        }
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !worker.poll(&mut app).await {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(app.daemon.pty_sessions.is_empty());
        if scenario == "success" {
            assert!(matches!(
                app.active_dialog(),
                Some(Dialog::TerminalLaunch(_))
            ));
            assert_eq!(
                app.kernel_debug.serial_preview.as_ref().unwrap().spec,
                fixture.spec
            );
        } else {
            assert!(!matches!(
                app.active_dialog(),
                Some(Dialog::TerminalLaunch(_))
            ));
            assert!(app.kernel_debug.serial_preview.is_none());
        }
        if scenario == "error" {
            assert!(
                matches!(app.active_dialog(), Some(Dialog::KernelDebug(d)) if d.error.as_deref().unwrap().contains("requires"))
            );
        }
    }
}

#[tokio::test]
async fn kernel_debug_worker_installs_typed_tool_presence_without_process_launch() {
    use yoctui_model::{Action, App, Effect, KernelDebugAction as A, Screen};
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    let effect = yoctui_model::update(&mut app, Action::KernelDebug(A::Open)).unwrap();
    assert!(matches!(effect, Effect::KernelDebug(_)));
    let mut io = KernelDebugIo::default();
    io.submit(effect);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !io.poll(&mut app).await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(app.kernel_debug.tools.is_some());
    assert!(app.kernel_debug.pending.is_none());
    assert!(app.daemon.pty_sessions.is_empty());
}

#[cfg(unix)]
#[test]
fn kernel_debug_fake_ssh_preserves_typed_argv_without_local_shell_evaluation() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!(
        "yoctui-kernel-debug-argv-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let program = root.join("fake-ssh");
    fs::write(&program, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
    let tools = KernelDebugTools {
        cwd: root.clone(),
        programs: [("ssh".into(), program)].into(),
    };
    let mut draft = KernelDebugDraft::new(KernelDebugTool::Bpftrace);
    draft.host = "example.invalid".into();
    let request = prepare(&draft, &tools).unwrap();
    let output = std::process::Command::new(&request.program)
        .args(&request.arguments)
        .current_dir(&request.cwd)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .collect::<Vec<_>>(),
        request
            .arguments
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        request.arguments.last().unwrap(),
        "exec 'bpftrace' '-e' 'tracepoint:syscalls:sys_enter_openat { @[comm] = count(); }'"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn kernel_debug_discovery_is_presence_only_and_planning_runs_no_tool() {
    let tools = discover().unwrap();
    assert!(tools.cwd.is_dir());
    for program in tools.programs.values() {
        assert!(program.is_absolute());
        assert!(crate::terminal_launcher::executable(
            &fs::metadata(program).unwrap()
        ));
    }
    let mut draft = KernelDebugDraft::new(KernelDebugTool::Strace);
    draft.host = "example.invalid".into();
    draft.pid = "123".into();
    // A nonexistent DNS name is accepted only as a plan, never connected here.
    if tools.programs.contains_key("ssh") {
        let request = prepare(&draft, &tools).unwrap();
        assert!(request.arguments.iter().any(|arg| arg == "example.invalid"));
    }
}

#[test]
fn kernel_debug_preparation_rejects_missing_and_special_offline_files() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-kernel-debug-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let program = std::env::current_exe().unwrap();
    let tools = KernelDebugTools {
        cwd: root.clone(),
        programs: [("gdb".into(), program)].into(),
    };
    let mut draft = KernelDebugDraft::new(KernelDebugTool::GdbCore);
    draft.symbols = root.join("vmlinux").display().to_string();
    draft.data = root.join("core").display().to_string();
    assert!(prepare(&draft, &tools).is_err());
    fs::write(&draft.symbols, "fixture").unwrap();
    fs::write(&draft.data, "fixture").unwrap();
    assert!(prepare(&draft, &tools).is_ok());
    draft.data = root.display().to_string();
    assert!(prepare(&draft, &tools).is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&draft.symbols, root.join("link")).unwrap();
        draft.data = root.join("link").display().to_string();
        assert!(prepare(&draft, &tools).is_err());
    }
    let mut unavailable = tools.clone();
    unavailable
        .programs
        .insert("gdb".into(), root.join("missing"));
    assert!(prepare(&draft, &unavailable).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn kernel_debug_managed_preparation_requires_explicit_boot_files_without_launching() {
    let tools = discover().unwrap();
    assert!(tools.programs.contains_key("yoctui"));
    let mut draft = KernelDebugDraft::new(KernelDebugTool::QemuGdb);
    draft.qemu.runqemu = std::env::current_exe().unwrap().display().to_string();
    draft.qemu.build_dir = tools.cwd.display().to_string();
    assert!(prepare(&draft, &tools).is_err());
    draft.qemu.qemuboot = "/missing/image.qemuboot.conf".into();
    draft.qemu.kernel = "/missing/bzImage".into();
    draft.qemu.rootfs = "/missing/image.ext4".into();
    draft.symbols = "/missing/vmlinux".into();
    assert!(prepare(&draft, &tools).is_err());
}
