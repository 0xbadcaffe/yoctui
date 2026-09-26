pub(crate) fn concept_rootfs_app() -> App {
    let mut app = concept_idle_dashboard_app();
    app.screen = Screen::Images;
    app.navigator_selection = 5;
    app.focus = FocusTarget::Workspace;
    app.build.target = Some("core-image-minimal".into());
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: "core-image-minimal".into(),
        version: Some("1.0".into()),
        layer: Some("poky".into()),
        ..Default::default()
    });
    let path = PathBuf::from(
        "/workspace/yocto/build/tmp/deploy/images/qemux86-64/core-image-minimal-qemux86-64.rootfs.ext4",
    );
    let artifact = yoctui_model::ImageArtifact {
        identity: yoctui_model::ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: path.clone(),
        },
        kind: yoctui_model::ImageArtifactKind::RootFilesystem,
        size_bytes: ImageArtifactField::Available(184 * 1024 * 1024),
        modified_unix_seconds: ImageArtifactField::Available(1_777_231_023),
        checksums: ImageArtifactField::Available(vec![yoctui_model::ImageChecksum {
            algorithm: "sha256".into(),
            digest: "6d8d5e7d0f5546a0".into(),
            source: PathBuf::from(
                "/workspace/yocto/build/tmp/deploy/images/qemux86-64/core-image-minimal.sha256",
            ),
        }]),
        manifests: ImageArtifactField::Available(vec![PathBuf::from(
            "/workspace/yocto/build/tmp/deploy/images/qemux86-64/core-image-minimal.manifest",
        )]),
        licenses: ImageArtifactField::Available(vec![PathBuf::from(
            "/workspace/yocto/build/tmp/deploy/licenses/core-image-minimal/license.manifest",
        )]),
        spdx: ImageArtifactField::Available(vec![PathBuf::from(
            "/workspace/yocto/build/tmp/deploy/images/qemux86-64/core-image-minimal.spdx.json",
        )]),
        wic_files: ImageArtifactField::Available(Vec::new()),
    };
    let rootfs_image = artifact.identity.clone();
    app.image_artifact_selection = Some(artifact.identity.clone());
    app.image_artifacts = ImageArtifactInventoryState::Available {
        request: yoctui_model::ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        },
        inventory: yoctui_model::ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: ImageArtifactField::Available(PathBuf::from(
                "/workspace/yocto/build/tmp/deploy/images/qemux86-64",
            )),
            artifacts: vec![artifact],
        },
    };
    let category_specs = [
        ("base system", 90_u64, 46_800_000_u64),
        ("libraries", 110, 34_100_000),
        ("kernel and modules", 50, 18_900_000),
        ("locales", 70, 11_400_000),
        ("utilities", 70, 8_800_000),
        ("Other", 22, 6_400_000),
    ];
    let mut packages = Vec::new();
    for (category, count, bytes) in category_specs {
        for index in 0..count {
            packages.push(yoctui_model::RootfsInstalledPackage {
                identity: PackageIdentity::new(format!(
                    "{}-{index:03}",
                    category.replace(' ', "-")
                )),
                recipe: Some(category.replace(' ', "-")),
                category: category.into(),
                installed_size_bytes: bytes / count + u64::from(index < bytes % count),
                file_count: 3 + index % 19,
            });
        }
    }
    let selected_package = packages[0].identity.clone();
    let entries = vec![
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/".into()),
            kind: RootfsEntryKind::Directory,
            size_bytes: 0,
            package: None,
        },
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/bin".into()),
            kind: RootfsEntryKind::Directory,
            size_bytes: 0,
            package: None,
        },
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/bin/busybox".into()),
            kind: RootfsEntryKind::RegularFile,
            size_bytes: 1_198_080,
            package: Some(selected_package.clone()),
        },
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/bin/sh".into()),
            kind: RootfsEntryKind::Symlink,
            size_bytes: 7,
            package: Some(selected_package.clone()),
        },
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/etc".into()),
            kind: RootfsEntryKind::Directory,
            size_bytes: 0,
            package: None,
        },
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/etc/os-release".into()),
            kind: RootfsEntryKind::RegularFile,
            size_bytes: 218,
            package: Some(selected_package.clone()),
        },
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/usr".into()),
            kind: RootfsEntryKind::Directory,
            size_bytes: 0,
            package: None,
        },
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/usr/lib".into()),
            kind: RootfsEntryKind::Directory,
            size_bytes: 0,
            package: None,
        },
        yoctui_model::RootfsEntry {
            identity: yoctui_model::RootfsPathIdentity("/dev/console".into()),
            kind: RootfsEntryKind::Other,
            size_bytes: 0,
            package: None,
        },
    ];
    let request = yoctui_model::RootfsCompositionRequest {
        generation: 1,
        image: rootfs_image.clone(),
    };
    app.rootfs_composition = RootfsCompositionState::Partial {
        request,
        composition: yoctui_model::RootfsComposition {
            image: rootfs_image,
            installed_packages: yoctui_model::RootfsAuthority::Available(
                yoctui_model::RootfsPackageInventory { packages },
            ),
            filesystem_tree: yoctui_model::RootfsAuthority::Partial {
                value: yoctui_model::RootfsFilesystemTree { entries },
                limitations: vec!["package ownership is partial".into()],
            },
            system_inventory: yoctui_model::RootfsAuthority::Available(
                yoctui_model::RootfsSystemInventory::default(),
            ),
            root_directory: None,
        },
        limitations: vec!["package ownership is partial".into()],
    };
    app.images_view = ImagesView::RootfsPackages;
    app.rootfs_group_selection = Some(RootfsGroupIdentity::Category("base system".into()));
    app.rootfs_package_selection = Some(selected_package);
    app.rootfs_entry_selection = Some(yoctui_model::RootfsPathIdentity("/bin/busybox".into()));
    app
}

pub(crate) fn concept_editor_menu_app() -> App {
    let mut app = concept_idle_dashboard_app();
    app.screen = Screen::Recipes;
    app.navigator_selection = 3;
    app.focus = FocusTarget::Dialog;
    app.dialogs.push_back(Dialog::RecipeEditor(RecipeEditor {
        recipe: "bash".into(),
        root: PathBuf::from("/workspace/yocto/meta/recipes-extended/bash"),
        files: vec![
            PathBuf::from("bash_5.2.bb"),
            PathBuf::from("files/0001-fix-build.patch"),
        ],
        file_inventory_truncated: false,
        selection: 0,
        focus: yoctui_model::RecipeEditorFocus::Document,
        language: yoctui_model::SourceLanguage::BitBake,
        document: {
            let mut document = yoctui_model::TextAreaState::new(
                concat!(
                    "SUMMARY = \"GNU Bourne Again Shell\"\n",
                    "LICENSE = \"GPL-3.0-only\"\n",
                    "SRC_URI = \"https://ftp.gnu.org/gnu/bash/bash-5.2.tar.gz\"\n",
                    "inherit autotools\n\n",
                    "do_install:append() {\n",
                    "    install -Dm755 ${WORKDIR}/bash ${D}${bindir}/bash\n",
                    "}\n",
                )
                .into(),
            );
            document.set_mode(yoctui_model::TextAreaMode::Insert);
            document.insert("BROKEN_OVERRIDE =\n");
            document
        },
        searching: false,
        pending_search_position: None,
    }));
    if let Some(Dialog::RecipeEditor(editor)) = app.dialogs.back_mut() {
        editor.refresh_language_and_validation();
    }
    let _ = update(&mut app, Action::OpenApplicationMenu);
    let _ = update(&mut app, Action::SelectMenuGroup { delta: 1 });
    let _ = update(&mut app, Action::SelectMenuItem { delta: 2 });
    app
}

pub(crate) fn concept_terminal_sessions_app() -> App {
    let mut app = concept_idle_dashboard_app();
    app.screen = Screen::TerminalSessions;
    app.navigator_selection = 18;
    app.focus = FocusTarget::Workspace;
    app.terminal.client_id = Some([1; 16]);
    app.terminal.query = "busybox".into();
    let first = app.pane_layout.focused;
    app.pane_layout
        .split(first, SplitAxis::Vertical)
        .expect("concept fixture can split its terminal layout");
    app.pty_selection = 0;
    app.daemon.pty_sessions = [
        yoctui_model::ClientDaemonPtySummary {
            id: 1,
            name: "shell".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 2,
        },
        yoctui_model::ClientDaemonPtySummary {
            id: 2,
            name: "devshell:busybox".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 2,
        },
    ]
    .into();
    app.daemon.pty_details = vec![
        yoctui_model::ClientDaemonPtyDetails {
            id: 1,
            kind: yoctui_model::ClientDaemonPtyKind::BuildShell,
            cwd: "/workspace/yocto/build".into(),
            columns: 88,
            rows: 18,
            writer: Some([1; 16]),
            writer_epoch: 4,
            exit_code: None,
            restartable: true,
        },
        yoctui_model::ClientDaemonPtyDetails {
            id: 2,
            kind: yoctui_model::ClientDaemonPtyKind::Devshell,
            cwd: "/workspace/yocto/build".into(),
            columns: 88,
            rows: 18,
            writer: Some([2; 16]),
            writer_epoch: 7,
            exit_code: None,
            restartable: true,
        },
    ];
    app.daemon.pty_screens = vec![
        yoctui_model::ClientDaemonPtyScreen {
            session_id: 1,
            columns: 88,
            rows_count: 18,
            cursor_column: 35,
            cursor_row: 6,
            cursor_hidden: false,
            scrollback_offset: 0,
            rows: vec![
                "build-shell$ bitbake-layers show-layers".into(),
                "layer                 path".into(),
                "meta                  /workspace/yocto/meta".into(),
                "meta-poky             /workspace/yocto/meta-poky".into(),
                "meta-yocto-bsp        /workspace/yocto/meta-yocto-bsp".into(),
                "build-shell$".into(),
            ],
            cells: Vec::new(),
            scrollback_lines: 0,
            dropped_line_feeds_lower_bound: 0,
        },
        yoctui_model::ClientDaemonPtyScreen {
            session_id: 2,
            columns: 88,
            rows_count: 18,
            cursor_column: 18,
            cursor_row: 5,
            cursor_hidden: false,
            scrollback_offset: 0,
            rows: vec![
                "busybox-devshell$ make CONFIG_PREFIX=/tmp/rootfs".into(),
                "CC      coreutils/ls.o".into(),
                "CC      coreutils/cp.o".into(),
                "LD      busybox_unstripped".into(),
                "busybox-devshell$".into(),
            ],
            cells: Vec::new(),
            scrollback_lines: 312,
            dropped_line_feeds_lower_bound: 14,
        },
    ];
    if let Some(telemetry) = app.daemon.telemetry.as_mut() {
        telemetry.pty_sessions = 2;
    }
    app
}

pub(crate) fn readme_platform_app(component: yoctui_model::PlatformComponent) -> App {
    let mut app = concept_idle_dashboard_app();
    app.focus = FocusTarget::Workspace;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Running;
    let (screen, navigator_selection, target, provider, root, files) = match component {
        yoctui_model::PlatformComponent::Kernel => (
            Screen::Kernel,
            6,
            "virtual/kernel",
            "/workspace/yocto/meta-freescale/recipes-kernel/linux/linux-imx_6.6.bb",
            "/workspace/yocto/build/tmp/work/imx8mp_lpddr4_evk-poky-linux/linux-imx/6.6/source",
            vec![
                (".config", yoctui_model::PlatformFileKind::DotConfig, 96_418),
                (
                    "arch/arm64/boot/dts/freescale/imx8mp-evk.dts",
                    yoctui_model::PlatformFileKind::Dts,
                    18_304,
                ),
                (
                    "arch/arm64/boot/dts/freescale/imx8mp.dtsi",
                    yoctui_model::PlatformFileKind::Dtsi,
                    37_812,
                ),
                (
                    "deploy/imx8mp-evk.dtb",
                    yoctui_model::PlatformFileKind::Dtb,
                    41_996,
                ),
            ],
        ),
        yoctui_model::PlatformComponent::UBoot => (
            Screen::Firmware,
            7,
            "u-boot-fslc",
            "/workspace/yocto/meta-freescale/recipes-bsp/u-boot/u-boot-fslc_2024.01.bb",
            "/workspace/yocto/build/tmp/work/imx8mp_lpddr4_evk-poky-linux/u-boot-fslc/2024.01/source",
            vec![
                (".config", yoctui_model::PlatformFileKind::DotConfig, 51_202),
                (
                    "arch/arm/dts/imx8mp-evk.dts",
                    yoctui_model::PlatformFileKind::Dts,
                    12_880,
                ),
                (
                    "arch/arm/dts/imx8mp.dtsi",
                    yoctui_model::PlatformFileKind::Dtsi,
                    29_241,
                ),
                (
                    "build/imx8mp-evk.dtb",
                    yoctui_model::PlatformFileKind::Dtb,
                    27_604,
                ),
            ],
        ),
        _ => unreachable!("README gallery has explicit Kernel and U-Boot fixtures"),
    };
    app.workspace
        .variables
        .insert("MACHINE".into(), "imx8mp-lpddr4-evk".into());
    let root = PathBuf::from(root);
    let inventory = yoctui_model::PlatformInventory {
        component,
        target: target.into(),
        provider: Some(provider.into()),
        tasks: vec!["do_menuconfig".into(), "do_compile".into()],
        roots: vec![root.clone()],
        files: files
            .into_iter()
            .map(|(path, kind, size_bytes)| yoctui_model::PlatformFile {
                path: root.join(path),
                root: root.clone(),
                kind,
                size_bytes,
            })
            .collect(),
        dtc: Some("/usr/bin/dtc".into()),
        limitations: Vec::new(),
    };
    app.screen = screen;
    app.navigator_selection = navigator_selection;
    let workbench = if component == yoctui_model::PlatformComponent::Kernel {
        &mut app.kernel
    } else {
        &mut app.firmware
    };
    workbench.view = yoctui_model::PlatformView::DeviceTrees;
    workbench.inventory = PlatformInventoryState::Available(inventory);
    app
}

pub(crate) fn readme_device_tree_editor_app() -> App {
    let mut app = readme_platform_app(yoctui_model::PlatformComponent::Kernel);
    let root = PathBuf::from(
        "/workspace/yocto/build/tmp/work/imx8mp_lpddr4_evk-poky-linux/linux-imx/6.6/source",
    );
    let file = PathBuf::from("arch/arm64/boot/dts/freescale/imx8mp-evk.dts");
    app.focus = FocusTarget::Dialog;
    app.dialogs.push_back(Dialog::RecipeEditor(RecipeEditor {
        recipe: "Kernel device tree".into(),
        root,
        files: vec![file],
        file_inventory_truncated: false,
        selection: 0,
        focus: yoctui_model::RecipeEditorFocus::Document,
        language: yoctui_model::SourceLanguage::DeviceTree,
        document: {
            let mut document = yoctui_model::TextAreaState::new(
                include_str!("../../../tests/fixtures/device-tree/imx8mp-evk.dts")
                    .replace('\t', "    "),
            );
            document.select_position(0, 0, false);
            document
        },
        searching: false,
        pending_search_position: None,
    }));
    if let Some(Dialog::RecipeEditor(editor)) = app.dialogs.back_mut() {
        editor.refresh_language_and_validation();
    }
    app
}
