pub(crate) fn readme_systemd_services_app() -> App {
    let mut app = concept_rootfs_app();
    app.images_view = ImagesView::SystemdServices;
    app.zoomed_pane = Some(FocusTarget::Workspace);
    let root = PathBuf::from(
        "/workspace/yocto/build/tmp/work/qemux86_64-poky-linux/core-image-minimal/1.0/rootfs",
    );
    let services = [
        (
            "dbus.service",
            "D-Bus System Message Bus",
            "",
            "multi-user.target.wants",
        ),
        (
            "dropbear.service",
            "Dropbear SSH server",
            "",
            "multi-user.target.wants",
        ),
        (
            "getty@tty1.service",
            "Getty on tty1",
            "",
            "getty.target.wants",
        ),
        (
            "serial-getty@ttyS0.service",
            "Serial Getty on ttyS0",
            "",
            "getty.target.wants",
        ),
        (
            "systemd-journald.service",
            "Journal Service",
            "",
            "sysinit.target.wants",
        ),
        (
            "systemd-logind.service",
            "User Login Management",
            "org.freedesktop.login1",
            "multi-user.target.wants",
        ),
        (
            "systemd-networkd.service",
            "Network Configuration",
            "org.freedesktop.network1",
            "multi-user.target.wants",
        ),
        (
            "systemd-resolved.service",
            "Network Name Resolution",
            "org.freedesktop.resolve1",
            "multi-user.target.wants",
        ),
        (
            "systemd-timesyncd.service",
            "Network Time Synchronization",
            "",
            "sysinit.target.wants",
        ),
        (
            "systemd-tmpfiles-clean.service",
            "Cleanup Temporary Directories",
            "",
            "",
        ),
        (
            "systemd-udevd.service",
            "Rule-based Device Events",
            "",
            "sysinit.target.wants",
        ),
    ]
    .into_iter()
    .map(|(name, description, bus, enabled)| {
        let logical = format!("/usr/lib/systemd/system/{name}");
        yoctui_model::RootfsSystemdService {
            name: name.into(),
            logical_path: yoctui_model::RootfsPathIdentity(logical.clone().into()),
            host_path: root.join(logical.trim_start_matches('/')),
            description: Some(description.into()),
            bus_name: (!bus.is_empty()).then(|| bus.into()),
            enabled_by: if enabled.is_empty() {
                Vec::new()
            } else {
                vec![enabled.into()]
            },
            preview: format!("[Unit]\nDescription={description}\n"),
            preview_truncated: false,
        }
    })
    .collect();
    if let RootfsCompositionState::Partial { composition, .. } = &mut app.rootfs_composition {
        composition.root_directory = Some(root);
        composition.system_inventory =
            yoctui_model::RootfsAuthority::Available(yoctui_model::RootfsSystemInventory {
                systemd_services: services,
                ..Default::default()
            });
    }
    app.rootfs_systemd_selection = 6;
    app
}

pub(crate) fn readme_system_dbus_app() -> App {
    let mut app = readme_systemd_services_app();
    app.images_view = ImagesView::SystemDbus;
    if let RootfsCompositionState::Partial { composition, .. } = &mut app.rootfs_composition {
        let root = composition.root_directory.clone().unwrap();
        if let yoctui_model::RootfsAuthority::Available(inventory) =
            &mut composition.system_inventory
        {
            inventory.dbus_services = [
                ("org.bluez", "bluetooth.service", "root", "/usr/libexec/bluetooth/bluetoothd"),
                ("org.freedesktop.Avahi", "avahi-daemon.service", "avahi", "/usr/sbin/avahi-daemon -s"),
                ("org.freedesktop.hostname1", "systemd-hostnamed.service", "root", "/usr/lib/systemd/systemd-hostnamed"),
                ("org.freedesktop.login1", "systemd-logind.service", "root", "/usr/lib/systemd/systemd-logind"),
                ("org.freedesktop.network1", "systemd-networkd.service", "systemd-network", "/usr/lib/systemd/systemd-networkd"),
                ("org.freedesktop.resolve1", "systemd-resolved.service", "systemd-resolve", "/usr/lib/systemd/systemd-resolved"),
                ("org.freedesktop.timedate1", "systemd-timedated.service", "root", "/usr/lib/systemd/systemd-timedated"),
            ].into_iter().map(|(name, unit, user, exec)| {
                let logical = format!("/usr/share/dbus-1/system-services/{name}.service");
                yoctui_model::RootfsDbusService {
                    name: name.into(),
                    logical_path: yoctui_model::RootfsPathIdentity(logical.clone().into()),
                    host_path: root.join(logical.trim_start_matches('/')),
                    exec: Some(exec.into()),
                    user: Some(user.into()),
                    systemd_service: Some(unit.into()),
                    policy_files: vec![yoctui_model::RootfsPathIdentity(format!("/usr/share/dbus-1/system.d/{name}.conf").into())],
                    preview: format!("[D-BUS Service]\nName={name}\nExec={exec}\nUser={user}\nSystemdService={unit}\n"),
                    preview_truncated: false,
                }
            }).collect();
        }
    }
    app.rootfs_dbus_selection = 4;
    app
}

pub(crate) fn readme_udev_rules_app() -> App {
    let mut app = readme_systemd_services_app();
    app.images_view = ImagesView::UdevRules;
    let rules = [
        (
            "/usr/lib/udev/rules.d/50-udev-default.rules",
            false,
            None,
            "# Default device permissions\nSUBSYSTEM==\"tty\", GROUP=\"tty\"\n",
        ),
        (
            "/usr/lib/udev/rules.d/60-persistent-serial.rules",
            false,
            Some("/etc/udev/rules.d/60-persistent-serial.rules"),
            "# Vendor serial rules overridden by the image configuration\n",
        ),
        (
            "/etc/udev/rules.d/60-persistent-serial.rules",
            false,
            None,
            concat!(
                "# USB debug adapter: stable console symlink and access permissions\n",
                "ACTION!=\"add\", GOTO=\"board_serial_end\"\n",
                "SUBSYSTEM!=\"tty\", GOTO=\"board_serial_end\"\n\n",
                "ATTRS{idVendor}==\"0403\", ATTRS{idProduct}==\"6011\", \\\n",
                "    SYMLINK+=\"board-console-%k\", GROUP=\"dialout\", MODE=\"0660\"\n\n",
                "LABEL=\"board_serial_end\"\n"
            ),
        ),
        ("/etc/udev/rules.d/80-net-setup-link.rules", true, None, ""),
        (
            "/usr/lib/udev/rules.d/99-systemd.rules",
            false,
            None,
            "# Device units\nSUBSYSTEM==\"tty\", TAG+=\"systemd\"\n",
        ),
    ];
    if let RootfsCompositionState::Partial { composition, .. } = &mut app.rootfs_composition
        && let yoctui_model::RootfsAuthority::Available(inventory) =
            &mut composition.system_inventory
    {
        inventory.udev_rules = rules
            .into_iter()
            .map(
                |(path, masked, overridden, preview)| yoctui_model::RootfsUdevRule {
                    name: path.rsplit('/').next().unwrap().into(),
                    logical_path: yoctui_model::RootfsPathIdentity(path.into()),
                    masked,
                    overridden_by: overridden
                        .map(|path| yoctui_model::RootfsPathIdentity(path.into())),
                    limitation: None,
                    preview: preview.into(),
                    preview_truncated: false,
                },
            )
            .collect();
    }
    app.rootfs_udev_selection = 2;
    app
}
