use super::*;

#[test]
fn concept_screen_contracts_render_through_production_renderer() {
    let mut active = literal_reference_app();
    active.navigator_selection = 9;
    active.focus = FocusTarget::Workspace;
    let scenes = [
        (
            "idle-dashboard",
            concept_idle_dashboard_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-idle-dashboard-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-idle-dashboard-160x50.cells"
            )),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-idle-dashboard-160x50.txt"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-idle-dashboard-160x50.txt"
            )),
        ),
        (
            "active-build-tasks",
            active,
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-active-build-tasks-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-active-build-tasks-160x50.cells"
            )),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-active-build-tasks-160x50.txt"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-active-build-tasks-160x50.txt"
            )),
        ),
        (
            "failed-build-errors",
            concept_failed_errors_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-failed-build-errors-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-failed-build-errors-160x50.cells"
            )),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-failed-build-errors-160x50.txt"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-failed-build-errors-160x50.txt"
            )),
        ),
        (
            "rootfs-composition",
            concept_rootfs_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-rootfs-composition-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-rootfs-composition-160x50.cells"
            )),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-rootfs-composition-160x50.txt"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-rootfs-composition-160x50.txt"
            )),
        ),
        (
            "editor-application-menu",
            concept_editor_menu_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-editor-application-menu-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-editor-application-menu-160x50.cells"
            )),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-editor-application-menu-160x50.txt"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-editor-application-menu-160x50.txt"
            )),
        ),
        (
            "terminal-sessions",
            concept_terminal_sessions_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-terminal-sessions-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-terminal-sessions-160x50.cells"
            )),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-terminal-sessions-160x50.txt"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/concept-terminal-sessions-160x50.txt"
            )),
        ),
    ];

    let update_goldens = std::env::var_os("YOCTUI_UPDATE_CONCEPT_GOLDENS").is_some();
    for (name, app, cell_path, cell_fixture, text_path, text_fixture) in scenes {
        let mut terminal =
            Terminal::new(TestBackend::new(TARGET_GOLDEN_WIDTH, TARGET_GOLDEN_HEIGHT)).unwrap();
        terminal
            .draw(|frame| render_at(frame, &app, literal_now()))
            .unwrap();
        let actual_cells = literal_cells(&terminal);
        let actual_text = concept_text_capture(&terminal);
        if update_goldens {
            fs::write(cell_path, serialize_target_golden(&actual_cells)).unwrap();
            fs::write(text_path, actual_text).unwrap();
        } else {
            assert_target_golden(name, &parse_target_golden(cell_fixture), &actual_cells);
            assert_eq!(
                text_fixture, actual_text,
                "concept screen {name} semantic capture changed; use the explicit update script only after reviewing the UI change"
            );
        }
    }
}

#[test]
fn readme_gallery_requested_workbenches_render_through_production_renderer() {
    let scenes = [
        (
            "kernel-device-tree",
            readme_platform_app(yoctui_model::PlatformComponent::Kernel),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-kernel-device-tree-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-kernel-device-tree-160x50.cells"
            )),
            ["Kernel", "Device trees", "imx8mp-evk.dts"].as_slice(),
        ),
        (
            "uboot-device-tree",
            readme_platform_app(yoctui_model::PlatformComponent::UBoot),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-uboot-device-tree-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-uboot-device-tree-160x50.cells"
            )),
            ["U-Boot", "Device trees", "imx8mp-evk.dts"].as_slice(),
        ),
        (
            "kernel-menuconfig",
            readme_menuconfig_app(true),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-kernel-menuconfig-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-kernel-menuconfig-160x50.cells"
            )),
            [
                "menuconfig:virtual/kernel",
                "Kernel Configuration",
                "Device Drivers",
            ]
            .as_slice(),
        ),
        (
            "uboot-menuconfig",
            readme_menuconfig_app(false),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-uboot-menuconfig-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-uboot-menuconfig-160x50.cells"
            )),
            [
                "menuconfig:u-boot-fslc",
                "U-Boot 2024.01 Configuration",
                "Boot options",
            ]
            .as_slice(),
        ),
        (
            "system-dbus",
            readme_system_dbus_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-system-dbus-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-system-dbus-160x50.cells"
            )),
            [
                "Offline system-bus activation map",
                "org.freedesktop.network1",
                "systemd-networkd.service",
                "/usr/lib/systemd/systemd-networkd",
                "Policies",
            ]
            .as_slice(),
        ),
        (
            "udev-rules",
            readme_udev_rules_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-udev-rules-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-udev-rules-160x50.cells"
            )),
            [
                "udev rules",
                "Overridden",
                "Masked",
                "Rule preview",
                "ATTRS{idVendor}",
                "board-console-%k",
            ]
            .as_slice(),
        ),
        (
            "systemd-services",
            readme_systemd_services_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-systemd-services-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-systemd-services-160x50.cells"
            )),
            [
                "Offline systemd service files",
                "systemd-networkd.service",
                "org.freedesktop.network1",
                "multi-user.target.wants",
                "disabled/static",
            ]
            .as_slice(),
        ),
        (
            "device-tree-editor",
            readme_device_tree_editor_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-device-tree-editor-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-device-tree-editor-160x50.cells"
            )),
            [
                "Recipe editor: Kernel device tree",
                "imx8mp-evk.dts",
                "/dts-v1/",
                "compatible",
                "NXP i.MX8MPlus EVK board",
                "hdmi_connector_in",
                "Device Tree",
            ]
            .as_slice(),
        ),
        (
            "device-tree-compile-options",
            readme_device_tree_compile_app(),
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-device-tree-compile-options-160x50.cells"
            ),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/golden/readme-device-tree-compile-options-160x50.cells"
            )),
            [
                "Compile device tree",
                "Generate symbols (-@)",
                "Output padding (-p)",
                "4096 bytes",
                "Enter review launch",
            ]
            .as_slice(),
        ),
    ];
    let update_goldens = std::env::var_os("YOCTUI_UPDATE_README_GOLDENS").is_some();
    for (name, app, cell_path, cell_fixture, anchors) in scenes {
        assert_eq!(app.navigator_screen(), app.screen);
        let mut terminal =
            Terminal::new(TestBackend::new(TARGET_GOLDEN_WIDTH, TARGET_GOLDEN_HEIGHT)).unwrap();
        terminal
            .draw(|frame| render_at(frame, &app, literal_now()))
            .unwrap();
        let actual_cells = literal_cells(&terminal);
        let actual_text = concept_text_capture(&terminal);
        for anchor in anchors {
            assert!(
                actual_text.contains(anchor),
                "{name} missing {anchor}: {actual_text}"
            );
        }
        if name == "device-tree-editor" {
            let buffer = terminal.backend().buffer();
            let token_color = |token: &str| {
                for y in 0..TARGET_GOLDEN_HEIGHT {
                    let row: String = (0..TARGET_GOLDEN_WIDTH)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect();
                    if let Some(offset) = row.find(token) {
                        let x = row[..offset].chars().count() as u16;
                        return buffer[(x, y)].fg;
                    }
                }
                panic!("DTS token not visible: {token}");
            };
            let colors = [
                "/dts-v1/",
                "model =",
                "chosen {",
                "\"NXP i.MX8MPlus EVK board\"",
                "* Copyright 2019 NXP",
            ]
            .map(token_color);
            for (i, color) in colors.iter().enumerate() {
                for other in &colors[i + 1..] {
                    assert_ne!(color, other, "DTS syntax roles must have distinct colors");
                }
            }
        }
        if update_goldens {
            fs::write(cell_path, serialize_target_golden(&actual_cells)).unwrap();
        } else {
            assert_target_golden(name, &parse_target_golden(cell_fixture), &actual_cells);
        }
    }
}

#[test]
fn concept_screens_keep_navigator_identity_aligned_with_the_visible_workspace() {
    let mut active = literal_reference_app();
    active.navigator_selection = 9;
    for app in [
        concept_idle_dashboard_app(),
        active,
        concept_failed_errors_app(),
        concept_rootfs_app(),
        concept_editor_menu_app(),
        concept_terminal_sessions_app(),
    ] {
        assert_eq!(app.navigator_screen(), app.screen);
    }
}

#[test]
fn literal_reference_cell_and_style_golden() {
    let app = literal_reference_app();
    let mut terminal = Terminal::new(TestBackend::new(LITERAL_WIDTH, LITERAL_HEIGHT)).unwrap();
    terminal
        .draw(|frame| render_at(frame, &app, literal_now()))
        .unwrap();
    let actual = literal_cells(&terminal);
    if std::env::var_os("YOCTUI_UPDATE_LITERAL_GOLDEN").is_some() {
        fs::create_dir_all(
            PathBuf::from(LITERAL_GOLDEN_PATH)
                .parent()
                .expect("golden parent"),
        )
        .unwrap();
        fs::write(LITERAL_GOLDEN_PATH, serialize_literal_golden(&actual)).unwrap();
        return;
    }
    let expected = parse_literal_golden(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/golden/literal-reference-160x48.cells"
    )));
    assert_literal_cells(&expected, &actual);
}
