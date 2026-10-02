use super::*;
use yoctui_model::{
    KernelDebugDialog, KernelDebugDraft, KernelDebugTool, KernelDebugTools,
    TerminalLaunchDestination, TerminalLaunchDialog,
};

fn app() -> App {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    app.focus = FocusTarget::Workspace;
    app.kernel_debug.visible = true;
    app.kernel_debug.tools = Some(KernelDebugTools {
        cwd: "/work".into(),
        programs: [
            ("gdb".into(), "/tools/gdb".into()),
            ("ssh".into(), "/tools/ssh".into()),
        ]
        .into(),
    });
    app
}

#[test]
fn kernel_debug_flash_mode_and_review_show_private_boot_without_kernel_argument() {
    let mut app = app();
    let tools = app.kernel_debug.tools.as_mut().unwrap();
    tools
        .programs
        .insert("runqemu".into(), "/tools/runqemu".into());
    tools
        .programs
        .insert("yoctui".into(), "/tools/yoctui".into());
    let mut draft = KernelDebugDraft::new(KernelDebugTool::QemuGdb);
    draft.qemu.boot_mode = yoctui_model::QemuDebugBootMode::OpenBmcRomulusFlash;
    draft.qemu.build_dir = "/work".into();
    draft.qemu.qemuboot = "/work/romulus.qemuboot.conf".into();
    draft.qemu.kernel = "/work/zImage".into();
    draft.qemu.rootfs = "/work/romulus.static.mtd".into();
    draft.symbols = "/work/vmlinux".into();
    app.dialogs
        .push_back(Dialog::KernelDebug(KernelDebugDialog {
            draft: draft.clone(),
            selection: 7,
            guide_scroll: 0,
            error: None,
        }));
    let form = rendered_text(&app, 160, 42);
    assert!(
        form.contains("Boot mode (Left/Right/Space): OpenBMC Romulus flash"),
        "{form}"
    );
    let tools = app.kernel_debug.tools.as_ref().unwrap();
    let request = draft.plan(tools).unwrap();
    app.kernel_debug.qemu_preview = Some(draft.qemu_spec(tools).unwrap());
    app.kernel_debug.prepared = Some(request.clone());
    app.dialogs.clear();
    app.dialogs
        .push_back(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request,
            destination: TerminalLaunchDestination::Embedded,
            output_must_not_exist: None,
        }));
    let review = rendered_text(&app, 160, 42);
    for text in [
        "OpenBMC Romulus flash",
        "NOT a launch argument",
        "hbreak start_kernel",
        "private copy",
        "original flash",
        "/PRIVATE_SESSION/romulus.static.mtd",
    ] {
        assert!(review.contains(text), "{text}: {review}");
    }
    assert!(!review.contains("bootparams=nokaslr"));
    assert!(!review.contains("argv[0]: \"/work/zImage\""));
    for (width, height) in [(100, 30), (80, 24), (60, 15), (20, 5), (1, 1)] {
        let _ = rendered_text(&app, width, height);
    }
}

#[test]
fn kernel_debug_instrumentation_form_and_scrollable_exact_review_render_safely() {
    let mut app = app();
    app.kernel_debug.selection = 13;
    assert!(rendered_text(&app, 160, 42).contains("Config prep"));
    let mut draft = KernelDebugDraft::new(KernelDebugTool::Sanitizers);
    draft.instrumentation.config = "/work/exact/.config".into();
    draft.instrumentation.output = "/work/approved.cfg".into();
    app.dialogs
        .push_front(Dialog::KernelDebug(KernelDebugDialog {
            draft: draft.clone(),
            selection: 2,
            guide_scroll: 0,
            error: None,
        }));
    let text = rendered_text(&app, 160, 42);
    for value in [
        "CONFIG PREP",
        "KASAN generic",
        "NEW absolute .cfg",
        "approved.cfg",
        "Enter inspect",
        "no write",
        "Esc cancel",
    ] {
        assert!(text.contains(value), "{value}: {text}");
    }
    app.kernel_debug.instrumentation_preview = Some(yoctui_model::KernelInstrumentationPreview {
        draft: *draft.instrumentation.clone(),
        report: yoctui_model::KernelInstrumentationReport::inspect(
            draft.instrumentation.preset,
            "CONFIG_KASAN=m\n# CONFIG_KCSAN is not set\n",
        )
        .unwrap(),
        destination_parent: "/work".into(),
        parent_identity: None,
    });
    let text = rendered_text(&app, 160, 42);
    for value in [
        "NEEDS RESOLUTION",
        "observed m; requested y",
        "observed n; requested n",
        "absent/unknown",
        "NOT proven",
        "Enter export NEW .cfg",
        "Esc back to edit",
    ] {
        assert!(text.contains(value), "{value}: {text}");
    }
    let Some(Dialog::KernelDebug(d)) = app.active_dialog_mut() else {
        panic!()
    };
    d.guide_scroll = 19;
    let text = rendered_text(&app, 160, 42);
    assert!(text.contains("Exact requested fragment"), "{text}");
    assert!(text.contains("CONFIG_KASAN_GENERIC=y"), "{text}");
    for (w, h) in [(80, 24), (60, 15), (20, 5), (1, 1)] {
        let _ = rendered_text(&app, w, h);
    }
}

#[test]
fn kernel_debug_instrumentation_lockdep_match_and_export_pending_never_claim_live_session() {
    let mut app = app();
    let mut draft = KernelDebugDraft::new(KernelDebugTool::Lockdep);
    draft.instrumentation.config = "/work/.config".into();
    draft.instrumentation.output = "/work/lockdep.cfg".into();
    let preview = yoctui_model::KernelInstrumentationPreview {
        draft: *draft.instrumentation.clone(),
        report: yoctui_model::KernelInstrumentationReport::inspect(
            draft.instrumentation.preset,
            &draft.instrumentation.preset.fragment(),
        )
        .unwrap(),
        destination_parent: "/work".into(),
        parent_identity: None,
    };
    app.dialogs
        .push_front(Dialog::KernelDebug(KernelDebugDialog {
            draft,
            selection: 0,
            guide_scroll: 0,
            error: None,
        }));
    app.kernel_debug.instrumentation_preview = Some(preview.clone());
    let text = rendered_text(&app, 160, 42);
    assert!(text.contains("CONFIG MATCH (this file only)"), "{text}");
    assert!(text.contains("NOT runtime readiness"), "{text}");
    app.kernel_debug.pending = Some(yoctui_model::KernelDebugOperation::ExportInstrumentation {
        expected: Box::new(preview),
    });
    let text = rendered_text(&app, 160, 42);
    assert!(text.contains("wait for result"), "{text}");
    assert!(text.contains("input locked"), "{text}");
    assert!(!text.contains("Esc cancel"), "{text}");
    for (w, h) in [(80, 24), (20, 5), (1, 1)] {
        let _ = rendered_text(&app, w, h);
    }
}

#[test]
fn kernel_debug_catalogue_and_target_scope_are_visible_without_metadata() {
    let mut app = app();
    let output = rendered_text(&app, 160, 42);
    assert!(output.contains("3 Debugging"), "{output}");
    for label in ["GDB", "strace", "perf", "bpftrace", "SysRq", "Guide only"] {
        assert!(output.contains(label), "{label}: {output}");
    }
    app.dialogs
        .push_back(Dialog::KernelDebug(KernelDebugDialog {
            draft: KernelDebugDraft::new(KernelDebugTool::Strace),
            selection: 1,
            guide_scroll: 0,
            error: Some("Host is required".into()),
        }));
    let output = rendered_text(&app, 160, 42);
    assert!(output.contains("SSH TARGET"));
    assert!(output.contains("Host is required"));
    assert!(output.contains("PID on selected system"));
    assert!(output.contains("Enter review exact launch"));
    assert!(output.contains("NOT a kernel source debugger"));
    for (width, height) in [(80, 24), (60, 15), (20, 5), (1, 1)] {
        let _ = rendered_text(&app, width, height);
    }
}

#[test]
fn kernel_debug_serial_form_and_config_command_review_keep_board_scope_explicit() {
    let mut app = app();
    app.kernel_debug
        .tools
        .as_mut()
        .unwrap()
        .programs
        .insert("yoctui".into(), "/tools/yoctui".into());
    let mut draft = KernelDebugDraft::new(KernelDebugTool::KgdbSerial);
    draft.symbols = "/work/vmlinux".into();
    draft.serial = yoctui_model::KgdbSerialDraft {
        config: "/work/.config".into(),
        device: "/dev/ttyUSB0".into(),
        target_uart: "ttyAMA0".into(),
        ready: "yes".into(),
        ..Default::default()
    };
    app.dialogs
        .push_front(Dialog::KernelDebug(KernelDebugDialog {
            draft: draft.clone(),
            selection: 5,
            guide_scroll: 0,
            error: None,
        }));
    let text = rendered_text(&app, 160, 42);
    for value in [
        "PHYSICAL BOARD",
        "Exact target kernel .config",
        "Host serial character tty device",
        "Target UART name",
        "configured/halted",
        "yes_",
        "Enter review",
        "Esc cancel",
    ] {
        assert!(text.contains(value), "{value}: {text}");
    }
    for (w, h) in [(80, 24), (60, 15), (20, 5), (1, 1)] {
        let _ = rendered_text(&app, w, h);
    }
    let tools = app.kernel_debug.tools.as_ref().unwrap();
    let request = draft.plan(tools).unwrap();
    app.kernel_debug.serial_preview = Some(yoctui_model::KgdbSerialPreview { spec: draft.serial_spec(tools).unwrap(), report: yoctui_model::KgdbConfigReport::inspect("CONFIG_KGDB=y\nCONFIG_KGDB_SERIAL_CONSOLE=y\nCONFIG_DEBUG_INFO=y\nCONFIG_STRICT_KERNEL_RWX=y\n").unwrap() });
    app.kernel_debug.prepared = Some(request.clone());
    app.dialogs.clear();
    app.dialogs
        .push_front(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request,
            destination: TerminalLaunchDestination::Embedded,
            output_must_not_exist: None,
        }));
    let text = rendered_text(&app, 160, 42);
    for value in [
        "BOARD TARGET",
        "CONFIG_KGDB=y",
        "CONFIG_KGDB_SERIAL_CONSOLE=y",
        "CONFIG_STRICT_KERNEL_RWX=y",
        "absent/unknown",
        "Ctrl+C cannot reliably",
        "SysRq-G",
        "kgdboc=ttyAMA0,115200",
        "Embedded in Yoctui",
        "Esc cancels without spawning",
    ] {
        assert!(text.contains(value), "{value}: {text}");
    }
    app.kernel_debug.preview_scroll = 17;
    let text = rendered_text(&app, 160, 42);
    assert!(text.contains("set serial baud 115200"), "{text}");
    assert!(text.contains("target remote /dev/ttyUSB0"), "{text}");
    assert!(!text.contains("__kgdb-serial-session"));
    for (w, h) in [(80, 24), (60, 15), (20, 5), (1, 1)] {
        let _ = rendered_text(&app, w, h);
    }
}

#[test]
fn kernel_debug_guide_and_exact_preview_keep_risks_and_cancel_visible() {
    let mut app = app();
    app.dialogs
        .push_back(Dialog::KernelDebug(KernelDebugDialog {
            draft: KernelDebugDraft::new(KernelDebugTool::SysrqKdump),
            selection: 0,
            guide_scroll: 0,
            error: None,
        }));
    let output = rendered_text(&app, 160, 42);
    assert!(output.contains("GUIDE ONLY"));
    assert!(output.contains("crash or reboot"));
    assert!(output.contains("Esc close"));
    let mut draft = KernelDebugDraft::new(KernelDebugTool::GdbRemote);
    draft.symbols = "/work/vmlinux".into();
    let request = draft
        .plan(app.kernel_debug.tools.as_ref().unwrap())
        .unwrap();
    app.kernel_debug.prepared = Some(request.clone());
    app.dialogs.clear();
    app.dialogs
        .push_back(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request,
            destination: TerminalLaunchDestination::Embedded,
            output_must_not_exist: None,
        }));
    let output = rendered_text(&app, 160, 42);
    for text in [
        "Review Kernel debugging launch",
        "HOST, NOT TARGET",
        "PAUSE",
        "set auto-load off",
        "set debuginfod enabled off",
        "target remote 127.0.0.1:1234",
        "Embedded in Yoctui",
        "Esc cancels without spawning",
    ] {
        assert!(output.contains(text), "{text}: {output}");
    }
    for (width, height) in [(80, 24), (60, 15), (20, 5), (1, 1)] {
        let _ = rendered_text(&app, width, height);
    }
}

#[test]
fn kernel_debug_managed_qemu_form_and_child_templates_render_without_json_parsing() {
    let mut app = app();
    let tools = app.kernel_debug.tools.as_mut().unwrap();
    tools
        .programs
        .insert("runqemu".into(), "/tools/runqemu".into());
    tools
        .programs
        .insert("yoctui".into(), "/tools/yoctui".into());
    let mut draft = KernelDebugDraft::new(KernelDebugTool::QemuGdb);
    draft.qemu.build_dir = "/work".into();
    draft.qemu.qemuboot = "/work/image.qemuboot.conf".into();
    draft.qemu.kernel = "/work/bzImage".into();
    draft.qemu.rootfs = "/work/image.ext4".into();
    draft.symbols = "/work/vmlinux".into();
    app.dialogs
        .push_back(Dialog::KernelDebug(KernelDebugDialog {
            draft: draft.clone(),
            selection: 6,
            guide_scroll: 0,
            error: None,
        }));
    let text = rendered_text(&app, 160, 42);
    assert!(text.contains("MANAGED HOST GUEST"));
    assert!(text.contains("Guest memory (MiB)"));
    let tools = app.kernel_debug.tools.as_ref().unwrap();
    let request = draft.plan(tools).unwrap();
    app.kernel_debug.qemu_preview = Some(draft.qemu_spec(tools).unwrap());
    app.kernel_debug.prepared = Some(request.clone());
    app.dialogs.clear();
    app.dialogs
        .push_back(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request,
            destination: TerminalLaunchDestination::Embedded,
            output_must_not_exist: None,
        }));
    let text = rendered_text(&app, 160, 42);
    for value in [
        "PRIVATE_SESSION",
        "placeholder",
        "QEMU child",
        "snapshot",
        "nonetwork",
        "nokaslr",
        "qemuparams=-S",
        "quit stops",
    ] {
        assert!(text.contains(value), "{value}: {text}");
    }
    app.kernel_debug.preview_scroll = 12;
    let text = rendered_text(&app, 160, 42);
    assert!(text.contains("GDB child"), "{text}");
    assert!(text.contains("set auto-load off"));
    for (w, h) in [(80, 24), (20, 5), (1, 1)] {
        let _ = rendered_text(&app, w, h);
    }
}
