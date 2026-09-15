//! Concept fixtures.
use super::*;

pub(crate) fn literal_reference_app() -> App {
    let mut app = App::new(512, 1024 * 1024);
    let source_dir = PathBuf::from("/workspace/yocto");
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Navigator;
    app.navigator_selection = 2;
    app.backend = "bridge".into();
    app.workspace.build_dir = Some(source_dir.join("build"));
    app.workspace.source_dir = Some(source_dir.clone());
    app.workspace.release = Some("scarthgap".into());
    app.workspace.bitbake_version = Some("2.8.0".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    app.workspace.layers = [
        "poky",
        "meta",
        "meta-poky",
        "meta-yocto-bsp",
        "meta-oe",
        "meta-python",
        "meta-networking",
    ]
    .into_iter()
    .enumerate()
    .map(|(index, name)| yoctui_model::Layer {
        name: name.into(),
        path: source_dir.join(name),
        priority: Some(index as i32 + 5),
    })
    .collect();
    app.workspace.recipes = ["busybox", "bash", "core-image-minimal"]
        .into_iter()
        .map(|name| yoctui_model::Recipe {
            name: name.into(),
            version: (name == "bash").then(|| "5.2.21".into()),
            layer: Some("poky".into()),
            ..Default::default()
        })
        .collect();
    app.available_images = vec![
        "core-image-minimal".into(),
        "core-image-full-cmdline".into(),
    ];
    app.build.status = BuildStatus::Running;
    app.build.target = Some("core-image-minimal".into());
    app.build.started = Some(literal_now() - Duration::from_secs(978));
    app.build.completed = 4;
    app.build.total = Some(10);
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Running;
    app.daemon.connected_clients = 1;
    app.daemon.telemetry = Some(yoctui_model::ClientDaemonTelemetry {
        uptime_seconds: 8_100,
        active_jobs: 1,
        pty_sessions: 1,
        queue_depth: 0,
        pressure: yoctui_model::ClientDaemonPressureCounters::default(),
        memory_bytes: Some(32 * 1024 * 1024),
        recovery: yoctui_model::DaemonRecoveryState::Recovered,
    });
    app.daemon.jobs.push(yoctui_model::ClientDaemonJobSummary {
        id: 858,
        kind: yoctui_model::ClientDaemonJobKind::BitBakeBuild,
        label: "core-image-minimal".into(),
        lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
        progress_current: Some(4),
        progress_total: Some(10),
        exit_code: None,
    });
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 1,
            name: "terminal".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    app.host_telemetry.cpu_utilization_percent = Some(24);
    app.host_telemetry.disk_available_bytes = Some(23 * 1024 * 1024 * 1024);

    for (index, task_name) in ["do_fetch", "do_unpack", "do_patch", "do_configure"]
        .into_iter()
        .enumerate()
    {
        app.completed_tasks.push_back(yoctui_model::CompletedTask {
            task: yoctui_model::TaskInfo {
                id: yoctui_model::TaskId(format!("bash:{task_name}")),
                recipe: "bash_5.2.21-2".into(),
                task: task_name.into(),
                progress: Some(100),
                state: yoctui_model::TaskState::Completed,
                started: Some(literal_now() - Duration::from_secs(600 - index as u64)),
                finished: Some(literal_now() - Duration::from_secs(599 - index as u64)),
                ..Default::default()
            },
            success: true,
        });
    }
    let active = yoctui_model::TaskInfo {
            id: yoctui_model::TaskId("bash:do_compile".into()),
            recipe: "bash_5.2.21-2".into(),
            task: "do_compile".into(),
            progress: Some(72),
            state: yoctui_model::TaskState::Active,
            worker: Some("worker-1".into()),
            pid: Some(35_421),
            started: Some(literal_now() - Duration::from_secs(590)),
            log_path: Some("/workspace/yocto/build/tmp/work/qemux86-64-poky-linux/bash/5.2.21-r2/temp/log.do_compile.85873".into()),
            ..Default::default()
        };
    app.tasks.insert(active.id.clone(), active);
    app.task_progress_scroll = 4;
    for message in [
        "NOTE: Executing Tasks",
        "NOTE: Started: do_compile",
        "|  CC     builtins/histfile.o",
        "|  CC     builtins/jobs.o",
        "|  CC     execute_cmd.o",
        "[ 72%] Linking bash",
    ] {
        let _ = update(
            &mut app,
            Action::Log(yoctui_model::LogEntry {
                id: 0,
                severity: yoctui_model::Severity::Info,
                message: message.into(),
                recipe: Some("bash_5.2.21-2".into()),
                task: Some("do_compile".into()),
                path: None,
                timestamp: literal_now(),
                build: Some("core-image-minimal".into()),
                protected: false,
                diagnostic: None,
            }),
        );
    }
    for (id, title, started_ago, finished_ago, succeeded) in [
        (854, "virtual/kernel", 1_269, 1_215, true),
        (855, "core-image-minimal", 1_194, 1_086, false),
        (856, "busybox", 1_005, 988, true),
        (857, "core-image-minimal", 977, 682, true),
    ] {
        let job_id = yoctui_model::BackgroundJobId(id);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
                id: job_id,
                kind: yoctui_model::BackgroundJobKind::Build,
                title: title.into(),
                context: yoctui_model::BackgroundJobContext {
                    target: Some(title.into()),
                    ..Default::default()
                },
                cancellation_supported: true,
                queued_at: literal_now() - Duration::from_secs(started_ago + 1),
            }),
        );
        let _ = update(
            &mut app,
            Action::StartBackgroundJob {
                id: job_id,
                started_at: literal_now() - Duration::from_secs(started_ago),
            },
        );
        let _ = update(&mut app, Action::RunBackgroundJob { id: job_id });
        let finished_at = literal_now() - Duration::from_secs(finished_ago);
        let _ = if succeeded {
            update(
                &mut app,
                Action::SucceedBackgroundJob {
                    id: job_id,
                    result: yoctui_model::BackgroundJobResult {
                        summary: "completed".into(),
                        artifacts: Vec::new(),
                    },
                    finished_at,
                },
            )
        } else {
            update(
                &mut app,
                Action::FailBackgroundJob {
                    id: job_id,
                    error: yoctui_model::BackgroundJobError {
                        summary: "build failed".into(),
                        detail: None,
                    },
                    finished_at,
                },
            )
        };
    }
    app
}

pub(crate) fn concept_idle_dashboard_app() -> App {
    let mut app = literal_reference_app();
    app.host_telemetry.cpu_utilization_percent = Some(18);
    app.host_telemetry.logical_cpu_count = Some(4);
    app.host_telemetry.memory_total_bytes = Some(16 * 1024 * 1024 * 1024);
    app.host_telemetry.memory_available_bytes = Some(9_964_324_126);
    app.host_telemetry.disk_total_bytes = Some(150 * 1024 * 1024 * 1024);
    app.host_telemetry.disk_available_bytes = Some(59_592_671_232);
    app.screen = Screen::Dashboard;
    app.navigator_selection = 0;
    app.focus = FocusTarget::Navigator;
    app.build.status = BuildStatus::Idle;
    app.build.started = None;
    app.build.completed = 0;
    app.build.total = None;
    app.tasks.clear();
    app.completed_tasks.clear();
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Exited;
    app.daemon.jobs.clear();
    app.daemon.pty_sessions.clear();
    app.daemon.pty_screens.clear();
    if let Some(telemetry) = app.daemon.telemetry.as_mut() {
        telemetry.active_jobs = 0;
        telemetry.pty_sessions = 0;
    }
    app
}

pub(crate) fn concept_failed_errors_app() -> App {
    let mut app = literal_reference_app();
    app.screen = Screen::Errors;
    app.navigator_selection = 11;
    app.focus = FocusTarget::Workspace;
    app.build.status = BuildStatus::Failed;
    app.build.exit_code = Some(1);
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Failed;
    if let Some(job) = app.daemon.jobs.first_mut() {
        job.lifecycle = yoctui_model::ClientDaemonLifecycle::Failed;
        job.exit_code = Some(1);
    }
    app.logs = yoctui_model::LogState::new(512, 1024 * 1024);
    let failed_task = app
        .tasks
        .get_mut(&yoctui_model::TaskId("bash:do_compile".into()))
        .expect("literal fixture has the selected compile task");
    failed_task.state = yoctui_model::TaskState::Failed;
    failed_task.progress = None;
    failed_task.finished = Some(literal_now());
    for (severity, message) in [
        (
            Severity::Info,
            "NOTE: running task bash:do_compile with oe_runmake",
        ),
        (
            Severity::Warning,
            "WARNING: bash:do_compile found a recoverable configure mismatch",
        ),
        (
            Severity::Error,
            "ERROR: bash:do_compile failed with exit code 1",
        ),
    ] {
        let _ = update(
            &mut app,
            Action::Log(yoctui_model::LogEntry {
                id: 0,
                severity,
                message: message.into(),
                recipe: Some("bash_5.2.21-2".into()),
                task: Some("do_compile".into()),
                path: Some("/workspace/yocto/build/tmp/log.do_compile.85873".into()),
                timestamp: literal_now(),
                build: Some("core-image-minimal".into()),
                protected: true,
                diagnostic: None,
            }),
        );
    }
    app.build.errors = 1;
    app.build.warnings = 1;
    app.error_selection = 1;
    app.logs.follow = false;
    app.logs.paused_len = Some(app.logs.entries.len());
    app.logs.query = "do_compile".into();
    app.logs.selection = app.logs.visible_count().saturating_sub(1);
    app.logs.dropped = 2;
    app.logs.dropped_warnings = 1;
    app.logs.dropped_errors = 1;
    app
}

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

pub(crate) fn readme_menuconfig_app(kernel: bool) -> App {
    let mut app = concept_idle_dashboard_app();
    app.screen = Screen::TerminalSessions;
    app.navigator_selection = 18;
    app.focus = FocusTarget::Workspace;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Running;
    app.workspace
        .variables
        .insert("MACHINE".into(), "imx8mp-lpddr4-evk".into());
    app.terminal.client_id = Some([1; 16]);
    let (name, cwd, title, entries) = if kernel {
        (
            "menuconfig:virtual/kernel",
            "/workspace/yocto/build/tmp/work/imx8mp_lpddr4_evk-poky-linux/linux-imx/6.6/build",
            "Linux/arm64 6.6 Kernel Configuration",
            vec![
                "General setup  --->",
                "Platform selection  --->",
                "Processor type and features  --->",
                "Power management options  --->",
                "Bus support  --->",
                "Executable file formats  --->",
                "Networking support  --->",
                "Device Drivers  --->",
                "File systems  --->",
                "Security options  --->",
                "Cryptographic API  --->",
            ],
        )
    } else {
        (
            "menuconfig:u-boot-fslc",
            "/workspace/yocto/build/tmp/work/imx8mp_lpddr4_evk-poky-linux/u-boot-fslc/2024.01/build",
            "U-Boot 2024.01 Configuration",
            vec![
                "Architecture select  --->",
                "General setup  --->",
                "Boot options  --->",
                "Command line interface  --->",
                "Device Drivers  --->",
                "File systems  --->",
                "Networking support  --->",
                "Security support  --->",
                "Library routines  --->",
                "Device Tree Control  --->",
                "Environment  --->",
            ],
        )
    };
    app.daemon.pty_sessions = vec![yoctui_model::ClientDaemonPtySummary {
        id: 1,
        name: name.into(),
        lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
        viewers: 1,
    }];
    app.daemon.pty_details = vec![yoctui_model::ClientDaemonPtyDetails {
        id: 1,
        kind: yoctui_model::ClientDaemonPtyKind::Menuconfig,
        cwd: cwd.into(),
        columns: 131,
        rows: 35,
        writer: Some([1; 16]),
        writer_epoch: 3,
        exit_code: None,
        restartable: true,
    }];
    app.daemon.pty_screens = vec![menuconfig_ncurses_screen(title, &entries)];
    if let Some(telemetry) = app.daemon.telemetry.as_mut() {
        telemetry.pty_sessions = 1;
    }
    app
}

fn menuconfig_ncurses_screen(title: &str, entries: &[&str]) -> yoctui_model::ClientDaemonPtyScreen {
    use std::fmt::Write as _;

    const COLUMNS: u16 = 131;
    const ROWS: u16 = 35;
    const LEFT: usize = 4;
    const DIALOG_WIDTH: usize = 123;
    let mut ansi = String::from("\x1b[37;44m\x1b[2J");
    let _ = write!(ansi, "\x1b[1;2H.config - {title}");
    let _ = write!(ansi, "\x1b[2;2H{}", "─".repeat(127));
    for row in 4..=33 {
        let _ = write!(
            ansi,
            "\x1b[{row};{LEFT}H\x1b[30;47m{}",
            " ".repeat(DIALOG_WIDTH)
        );
    }
    let _ = write!(
        ansi,
        "\x1b[4;{LEFT}H┌{}┐\x1b[33;{LEFT}H└{}┘",
        "─".repeat(DIALOG_WIDTH - 2),
        "─".repeat(DIALOG_WIDTH - 2)
    );
    for row in 5..33 {
        let _ = write!(ansi, "\x1b[{row};{LEFT}H│\x1b[{row};126H│");
    }
    let title_column = 4 + (DIALOG_WIDTH.saturating_sub(title.len() + 2) / 2);
    let _ = write!(
        ansi,
        "\x1b[4;{title_column}H\x1b[34;47;1m {title} \x1b[30;47;22m"
    );
    let instructions = [
        "Arrow keys navigate the menu.  <Enter> selects submenus --->  (or empty submenus ----).",
        "Highlighted letters are hotkeys.  Press <Y> includes, <N> excludes, <M> modularizes.",
        "Press <Esc><Esc> to exit, <?> for Help, </> for Search.  Legend: [*] built-in  [ ]",
    ];
    for (offset, line) in instructions.into_iter().enumerate() {
        let _ = write!(ansi, "\x1b[{};8H{line}", 6 + offset);
    }
    let _ = write!(ansi, "\x1b[10;8H┌{}┐", "─".repeat(113));
    for row in 11..28 {
        let _ = write!(ansi, "\x1b[{row};8H│\x1b[{row};122H│");
    }
    let _ = write!(ansi, "\x1b[28;8H└{}┘", "─".repeat(113));
    for (index, entry) in entries.iter().enumerate() {
        let row = 11 + index;
        if index == 0 {
            let _ = write!(
                ansi,
                "\x1b[{row};15H\x1b[37;44;1m {:<46}\x1b[30;47;22m",
                entry
            );
        } else {
            let _ = write!(ansi, "\x1b[{row};17H\x1b[34;47m{entry}\x1b[30;47m");
        }
    }
    let _ = write!(ansi, "\x1b[30;4H├{}┤", "─".repeat(DIALOG_WIDTH - 2));
    let _ = write!(
        ansi,
        "\x1b[31;17H\x1b[37;44;1m<Select>\x1b[31;47;22m    < Exit >    < Help >    < Save >    < Load >"
    );
    let _ = write!(ansi, "\x1b[11;15H\x1b[?25l");

    let dimensions = yoctui_model::PtyDimensions {
        columns: COLUMNS,
        rows: ROWS,
    };
    let mut emulator = yoctui_model::TerminalEmulator::new(dimensions, 0).unwrap();
    emulator.process(ansi.as_bytes()).unwrap();
    let snapshot = emulator.snapshot(0).unwrap();
    let cells = snapshot
        .cells
        .into_iter()
        .map(|cell| yoctui_model::ClientDaemonTerminalCell {
            contents: cell.contents,
            foreground: menuconfig_fixture_color(cell.foreground),
            background: menuconfig_fixture_color(cell.background),
            bold: cell.bold,
            dim: cell.dim,
            italic: cell.italic,
            underline: cell.underline,
            inverse: cell.inverse,
            wide: cell.wide,
            wide_continuation: cell.wide_continuation,
        })
        .collect();
    yoctui_model::ClientDaemonPtyScreen {
        session_id: 1,
        columns: COLUMNS,
        rows_count: ROWS,
        cursor_column: snapshot.cursor.1,
        cursor_row: snapshot.cursor.0,
        cursor_hidden: snapshot.modes.cursor_hidden,
        scrollback_offset: 0,
        rows: snapshot.plain_text.lines().map(str::to_owned).collect(),
        cells,
        scrollback_lines: 0,
        dropped_line_feeds_lower_bound: 0,
    }
}

fn menuconfig_fixture_color(
    color: yoctui_model::TerminalColor,
) -> yoctui_model::ClientDaemonTerminalColor {
    match color {
        yoctui_model::TerminalColor::Default => yoctui_model::ClientDaemonTerminalColor::Default,
        yoctui_model::TerminalColor::Indexed(index) => {
            let (red, green, blue) = match index {
                0 => (0, 0, 0),
                1 => (170, 0, 0),
                2 => (0, 170, 0),
                3 => (170, 85, 0),
                4 => (0, 0, 170),
                5 => (170, 0, 170),
                6 => (0, 170, 170),
                7 => (170, 170, 170),
                8 => (85, 85, 85),
                9 => (255, 85, 85),
                10 => (85, 255, 85),
                11 => (255, 255, 85),
                12 => (85, 85, 255),
                13 => (255, 85, 255),
                14 => (85, 255, 255),
                _ => (255, 255, 255),
            };
            yoctui_model::ClientDaemonTerminalColor::Rgb(red, green, blue)
        }
        yoctui_model::TerminalColor::Rgb(red, green, blue) => {
            yoctui_model::ClientDaemonTerminalColor::Rgb(red, green, blue)
        }
    }
}
