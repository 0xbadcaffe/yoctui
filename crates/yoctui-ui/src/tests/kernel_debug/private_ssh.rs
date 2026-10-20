use super::*;

#[test]
fn runtime_debug_inventory_reports_ssh_not_missing_host_tracers() {
    let mut app = app();
    app.kernel_debug.selection = 2;
    let output = rendered_text(&app, 160, 50);
    let strace = output
        .split_once("strace · syscall attach")
        .unwrap()
        .1
        .split('│')
        .next()
        .unwrap();
    assert!(strace.contains("SSH ready · target unchecked"), "{strace}");
    assert!(!strace.contains("Host missing"));
    app.kernel_debug
        .tools
        .as_mut()
        .unwrap()
        .programs
        .remove("ssh");
    let output = rendered_text(&app, 160, 50);
    let strace = output
        .split_once("strace · syscall attach")
        .unwrap()
        .1
        .split('│')
        .next()
        .unwrap();
    assert!(strace.contains("SSH client missing"), "{strace}");
}

#[test]
fn private_ssh_review_shows_exact_exposure_and_pause_lifetime() {
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
    draft.qemu.ssh_forward_port = "2245".into();
    draft.symbols = "/work/vmlinux".into();
    let spec = draft.qemu_spec(tools).unwrap();
    let request = draft.plan(tools).unwrap();
    app.kernel_debug.prepared = Some(request.clone());
    app.kernel_debug.qemu_preview = Some(spec);
    app.dialogs
        .push_back(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request,
            destination: TerminalLaunchDestination::Embedded,
            output_must_not_exist: None,
        }));
    let output = rendered_text(&app, 160, 50);
    assert!(output.contains("127.0.0.1:2245"), "{output}");
    assert!(
        output.contains("Guest outbound connections blocked"),
        "{output}"
    );
    assert!(output.contains("breakpoint pauses SSH"), "{output}");
    for (width, height) in [(80, 24), (100, 30), (20, 5)] {
        let _ = rendered_text(&app, width, height);
    }
}
