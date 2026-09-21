use super::*;

#[test]
fn kernel_menuconfig_uses_virtual_provider_and_requires_reported_task() {
    let mut app = App::new(10, 1_000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/work/build".into());
    app.kernel.inventory = PlatformInventoryState::Available(PlatformInventory {
        component: PlatformComponent::Kernel,
        target: "virtual/kernel".into(),
        provider: Some("/layers/linux.bb".into()),
        tasks: vec!["do_menuconfig".into()],
        roots: vec![],
        files: vec![],
        dtc: None,
        limitations: vec![],
    });
    let _ = update(&mut app, Action::LaunchKernelMenuconfig);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request: TerminalLaunchRequest { arguments, .. },
            destination: TerminalLaunchDestination::Embedded,
            ..
        })) if arguments == &vec!["bitbake", "virtual/kernel", "-c", "menuconfig"]
    ));
}
