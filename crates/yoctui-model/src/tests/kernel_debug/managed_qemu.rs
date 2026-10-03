#[test]
fn kernel_debug_managed_qemu_requires_inputs_and_preserves_typed_preview_until_confirmation() {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    app.kernel_debug.tools = Some(tools());
    app.kernel_debug.selection = 16;
    app.workspace.build_dir = Some("/work".into());
    update(&mut app, Action::KernelDebug(A::OpenSelected));
    let Some(Dialog::KernelDebug(dialog)) = app.active_dialog_mut() else {
        panic!()
    };
    assert_eq!(dialog.draft.qemu.build_dir, "/work");
    assert_eq!(dialog.draft.qemu.runqemu, "/tools/runqemu");
    assert!(dialog.draft.qemu.qemuboot.is_empty());
    assert_eq!(dialog.draft.fields().len(), 8);
    assert!(dialog.draft.plan(&tools()).is_err());
    dialog.draft.qemu.qemuboot = "/work/image.qemuboot.conf".into();
    dialog.draft.qemu.kernel = "/work/bzImage".into();
    dialog.draft.qemu.rootfs = "/work/image.ext4".into();
    dialog.draft.symbols = "/work/vmlinux".into();
    let Some(Effect::KernelDebug(request)) = update(&mut app, Action::KernelDebug(A::Review))
    else {
        panic!()
    };
    let Op::Prepare { draft, tools } = request.operation else {
        panic!()
    };
    let spec = draft.qemu_spec(&tools).unwrap();
    let prepared = draft.plan(&tools).unwrap();
    update(
        &mut app,
        Action::KernelDebug(A::Finished {
            generation: request.generation,
            result: Ok(R::Prepared(prepared.clone())),
        }),
    );
    assert_eq!(app.kernel_debug.qemu_preview, Some(spec));
    assert!(app.daemon.pty_sessions.is_empty());
    assert!(
        matches!(update(&mut app, Action::ConfirmTerminalLaunch), Some(Effect::Terminal(crate::TerminalEffect::Create { arguments, .. })) if arguments == prepared.arguments)
    );
}

#[test]
fn kernel_debug_flash_mode_selector_is_closed_and_cancel_launches_nothing() {
    let mut app = App::new(32, 4096);
    app.onboarding.open = false;
    app.screen = Screen::Kernel;
    app.kernel_debug.tools = Some(tools());
    app.kernel_debug.selection = 16;
    update(&mut app, Action::KernelDebug(A::OpenSelected));
    let Some(Dialog::KernelDebug(dialog)) = app.active_dialog_mut() else {
        panic!()
    };
    dialog.selection = 7;
    assert_eq!(dialog.draft.fields()[6], KernelDebugField::Memory);
    assert_eq!(dialog.draft.fields()[7], KernelDebugField::QemuBootMode);
    update(&mut app, Action::KernelDebug(A::ChangeScope));
    update(&mut app, Action::KernelDebug(A::Insert("arbitrary".into())));
    let Some(Dialog::KernelDebug(dialog)) = app.active_dialog() else {
        panic!()
    };
    assert_eq!(
        dialog.draft.qemu.boot_mode,
        crate::QemuDebugBootMode::OpenBmcRomulusFlash
    );
    assert!(app.daemon.pty_sessions.is_empty());
    update(&mut app, Action::KernelDebug(A::ChangeScope));
    let Some(Dialog::KernelDebug(dialog)) = app.active_dialog() else {
        panic!()
    };
    assert_eq!(
        dialog.draft.qemu.boot_mode,
        crate::QemuDebugBootMode::DirectKernel
    );
    assert!(update(&mut app, Action::KernelDebug(A::Cancel)).is_none());
    assert!(app.active_dialog().is_none());
    assert!(app.daemon.pty_sessions.is_empty());
}
