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
