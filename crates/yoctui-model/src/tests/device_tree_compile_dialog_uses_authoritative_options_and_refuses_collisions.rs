//! Regression tests for the shared platform device-tree compile flow.
use super::*;

fn inventory(component: PlatformComponent, source: PathBuf) -> PlatformInventoryState {
    PlatformInventoryState::Available(PlatformInventory {
        component,
        target: component.label().to_ascii_lowercase(),
        provider: None,
        tasks: Vec::new(),
        roots: vec![source.parent().unwrap().to_path_buf()],
        files: vec![PlatformFile {
            root: source.parent().unwrap().to_path_buf(),
            path: source,
            kind: PlatformFileKind::Dts,
            size_bytes: 64,
        }],
        dtc: Some(PathBuf::from("/toolchain/bin/dtc")),
        limitations: Vec::new(),
    })
}

fn unique_temp_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "yoctui-dtc-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn device_tree_compile_dialog_builds_exact_preview_for_kernel_and_firmware() {
    for (component, action) in [
        (PlatformComponent::Kernel, Action::CompileSelectedKernelDts),
        (PlatformComponent::UBoot, Action::CompileSelectedFirmwareDts),
    ] {
        let mut app = App::new(8, 512);
        let source = PathBuf::from(format!("/workspace/{}/board.dts", component.label()));
        let workbench = if component == PlatformComponent::Kernel {
            &mut app.kernel
        } else {
            &mut app.firmware
        };
        workbench.view = PlatformView::DeviceTrees;
        workbench.inventory = inventory(component, source.clone());

        assert_eq!(update(&mut app, action), None);
        let dialog = match app.active_dialog() {
            Some(Dialog::DtcCompile(dialog)) => dialog,
            other => panic!("expected dtc options, got {other:?}"),
        };
        assert_eq!(dialog.component, component);
        assert_eq!(dialog.source, source);

        let _ = update(&mut app, Action::AdjustDtcCompileOption { delta: 1 });
        let _ = update(&mut app, Action::SelectDtcCompileOption { delta: 2 });
        let _ = update(&mut app, Action::AdjustDtcCompileOption { delta: 1 });
        let _ = update(&mut app, Action::ConfirmDtcCompileOptions);

        let preview = match app.active_dialog() {
            Some(Dialog::TerminalLaunch(dialog)) => dialog,
            other => panic!("expected exact terminal preview, got {other:?}"),
        };
        assert_eq!(preview.request.program, PathBuf::from("/toolchain/bin/dtc"));
        assert_eq!(preview.request.kind, TerminalCreationKind::Utility);
        assert_eq!(
            preview.request.arguments,
            vec![
                "-I",
                "dts",
                "-O",
                "dtb",
                "-@",
                "-p",
                "256",
                "-o",
                &format!("/workspace/{}/board.yoctui.dtb", component.label()),
                &format!("/workspace/{}/board.dts", component.label()),
            ]
        );
        let effect = update(&mut app, Action::ConfirmTerminalLaunch);
        assert!(matches!(
            effect,
            Some(Effect::Terminal(TerminalEffect::Create {
                kind: TerminalCreationKind::Utility,
                ..
            }))
        ));
        assert!(app.active_dialog().is_none());
    }
}

#[test]
fn device_tree_compile_refuses_an_existing_derived_output() {
    let root = unique_temp_root("existing-output");
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("board.dts");
    let output = root.join("board.yoctui.dtb");
    std::fs::write(&output, b"existing").unwrap();

    let mut app = App::new(8, 512);
    app.kernel.view = PlatformView::DeviceTrees;
    app.kernel.inventory = inventory(PlatformComponent::Kernel, source);
    let _ = update(&mut app, Action::CompileSelectedKernelDts);

    assert!(app.active_dialog().is_none());
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("Refusing to overwrite"))
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn device_tree_compile_rejects_a_source_outside_its_authoritative_root() {
    let mut app = App::new(8, 512);
    let mut state = inventory(
        PlatformComponent::Kernel,
        PathBuf::from("/outside/board.dts"),
    );
    let PlatformInventoryState::Available(inventory) = &mut state else {
        unreachable!();
    };
    inventory.roots = vec![PathBuf::from("/workspace/kernel")];
    inventory.files[0].root = PathBuf::from("/workspace/kernel");
    app.kernel.view = PlatformView::DeviceTrees;
    app.kernel.inventory = state;

    assert_eq!(update(&mut app, Action::CompileSelectedKernelDts), None);
    assert!(app.active_dialog().is_none());
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("outside its authoritative root"))
    );
}

#[test]
fn device_tree_compile_reports_missing_dtc_plainly() {
    let mut app = App::new(8, 512);
    let mut state = inventory(
        PlatformComponent::Kernel,
        PathBuf::from("/workspace/kernel/board.dts"),
    );
    let PlatformInventoryState::Available(inventory) = &mut state else {
        unreachable!();
    };
    inventory.dtc = None;
    app.kernel.view = PlatformView::DeviceTrees;
    app.kernel.inventory = state;

    assert_eq!(update(&mut app, Action::CompileSelectedKernelDts), None);
    assert_eq!(
        app.notification.as_deref(),
        Some("Install the dtc compiler to work with device-tree binaries.")
    );
}

#[test]
fn opening_a_dtb_uses_decompile_and_reports_missing_dtc() {
    for (component, action) in [
        (PlatformComponent::Kernel, Action::OpenSelectedKernelFile),
        (PlatformComponent::UBoot, Action::OpenSelectedFirmwareFile),
    ] {
        let mut app = App::new(8, 512);
        let source = PathBuf::from(format!("/workspace/{}/board.dtb", component.label()));
        let mut state = inventory(component, source);
        let PlatformInventoryState::Available(inventory) = &mut state else {
            unreachable!();
        };
        inventory.files[0].kind = PlatformFileKind::Dtb;
        inventory.dtc = None;
        let workbench = if component == PlatformComponent::Kernel {
            &mut app.kernel
        } else {
            &mut app.firmware
        };
        workbench.view = PlatformView::DeviceTrees;
        workbench.inventory = state;

        assert_eq!(update(&mut app, action), None);
        assert!(app.active_dialog().is_none());
        assert_eq!(
            app.notification.as_deref(),
            Some("Install the dtc compiler to work with device-tree binaries.")
        );
    }
}

#[test]
fn opening_a_dtb_starts_the_decompile_flow_when_dtc_is_available() {
    let mut app = App::new(8, 512);
    let source = PathBuf::from("/workspace/kernel/board.dtb");
    let mut state = inventory(PlatformComponent::Kernel, source.clone());
    let PlatformInventoryState::Available(inventory) = &mut state else {
        unreachable!();
    };
    inventory.files[0].kind = PlatformFileKind::Dtb;
    app.kernel.view = PlatformView::DeviceTrees;
    app.kernel.inventory = state;

    assert_eq!(update(&mut app, Action::OpenSelectedKernelFile), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request: TerminalLaunchRequest {
                kind: TerminalCreationKind::Utility,
                arguments,
                ..
            },
            ..
        })) if arguments.last() == Some(&source.display().to_string())
    ));
}

#[cfg(unix)]
#[test]
fn device_tree_compile_refuses_a_dangling_output_symlink() {
    use std::os::unix::fs::symlink;

    let root = unique_temp_root("dangling-output");
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("board.dts");
    symlink(root.join("missing-target"), root.join("board.yoctui.dtb")).unwrap();

    let mut app = App::new(8, 512);
    app.kernel.view = PlatformView::DeviceTrees;
    app.kernel.inventory = inventory(PlatformComponent::Kernel, source);
    assert_eq!(update(&mut app, Action::CompileSelectedKernelDts), None);

    assert!(app.active_dialog().is_none());
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("Refusing to overwrite"))
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn device_tree_compile_rechecks_output_at_final_launch_confirmation() {
    let root = unique_temp_root("late-output");
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("board.dts");
    let output = root.join("board.yoctui.dtb");

    let mut app = App::new(8, 512);
    app.kernel.view = PlatformView::DeviceTrees;
    app.kernel.inventory = inventory(PlatformComponent::Kernel, source);
    assert_eq!(update(&mut app, Action::CompileSelectedKernelDts), None);
    assert_eq!(update(&mut app, Action::ConfirmDtcCompileOptions), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(_))
    ));

    std::fs::write(&output, b"appeared after preview").unwrap();
    assert_eq!(update(&mut app, Action::ConfirmTerminalLaunch), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(_))
    ));
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("Refusing to overwrite"))
    );
    std::fs::remove_dir_all(root).unwrap();
}
