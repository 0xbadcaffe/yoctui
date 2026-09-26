pub(crate) fn readme_device_tree_compile_app() -> App {
    let mut app = readme_platform_app(yoctui_model::PlatformComponent::Kernel);
    let file = app
        .kernel
        .selected_file()
        .expect("README Device Tree fixture has a selected DTS")
        .clone();
    let mut dialog = yoctui_model::DtcCompileDialog::new(
        yoctui_model::PlatformComponent::Kernel,
        &file,
        PathBuf::from("/usr/bin/dtc"),
    );
    dialog.symbols = true;
    dialog.sort = true;
    dialog.padding_bytes = 4_096;
    dialog.reserve_entries = 4;
    dialog.selection = 2;
    app.focus = FocusTarget::Dialog;
    app.dialogs.push_back(Dialog::DtcCompile(dialog));
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

    captured_terminal_screen(ansi.as_bytes(), COLUMNS, ROWS)
}

fn captured_terminal_screen(
    ansi: &[u8],
    columns: u16,
    rows: u16,
) -> yoctui_model::ClientDaemonPtyScreen {
    let dimensions = yoctui_model::PtyDimensions { columns, rows };
    let mut emulator = yoctui_model::TerminalEmulator::new(dimensions, 0).unwrap();
    emulator.process(ansi).unwrap();
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
        columns,
        rows_count: rows,
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

pub(crate) fn readme_repaired_workflow_app(scene: &str) -> App {
    let mut app = concept_idle_dashboard_app();
    app.source_git_status = yoctui_model::SourceGitStatus::Ready(yoctui_model::SourceGitSummary {
        branch: "master".into(),
        upstream: Some("origin/master".into()),
        ahead: 1,
        unstaged: 2,
        ..Default::default()
    });
    match scene {
        "cloning" => {
            app = App::new_unconfigured(512, 1024 * 1024);
            app.navigator_selection = 22;
            app.focus = FocusTarget::Workspace;
            update(
                &mut app,
                Action::SetBackgroundActivity {
                    activity: yoctui_model::BackgroundActivity::Cloning,
                    active: true,
                },
            );
        }
        "cancelling" => {
            app = literal_reference_app();
            app.screen = Screen::Tasks;
            app.navigator_selection = 9;
            app.focus = FocusTarget::Workspace;
            app.build.status = BuildStatus::Cancelling;
            update(
                &mut app,
                Action::SetBackgroundActivity {
                    activity: yoctui_model::BackgroundActivity::Cancelling,
                    active: true,
                },
            );
        }
        "search-empty" => {
            app.command_palette_open = true;
            app.command_palette_mode = CommandPaletteMode::GlobalRegexSearch;
            app.command_palette_query.clear();
        }
        "gitui-diff" | "gitui-commit" => {
            let ansi = if scene == "gitui-diff" {
                include_bytes!("../../../tests/fixtures/gitui-diff.ansi").as_slice()
            } else {
                include_bytes!("../../../tests/fixtures/gitui-commit.ansi").as_slice()
            };
            app.screen = Screen::TerminalSessions;
            app.navigator_selection = 18;
            app.focus = FocusTarget::Workspace;
            app.terminal.client_id = Some([1; 16]);
            app.daemon.pty_sessions = vec![yoctui_model::ClientDaemonPtySummary {
                id: 1,
                name: "GitUI".into(),
                lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
                viewers: 1,
            }];
            app.daemon.pty_details = vec![yoctui_model::ClientDaemonPtyDetails {
                id: 1,
                kind: yoctui_model::ClientDaemonPtyKind::Utility,
                cwd: "/workspace/yocto".into(),
                columns: 98,
                rows: 32,
                writer: Some([1; 16]),
                writer_epoch: 1,
                exit_code: None,
                restartable: true,
            }];
            app.daemon.pty_screens = vec![captured_terminal_screen(ansi, 98, 32)];
        }
        _ => panic!("unknown repaired workflow fixture: {scene}"),
    }
    app
}

pub(crate) fn readme_offline_history_app(scene: &str) -> App {
    use yoctui_model::{
        SavedBuild, SavedBuildLog, SavedBuildOutcome, SavedBuildTask, SavedBuildView,
    };
    let mut app = concept_idle_dashboard_app();
    app.require_daemon = true;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Disconnected;
    app.daemon.jobs.clear();
    let now = literal_now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let mut record = SavedBuild {
        id: "fixture-build-1".into(),
        target: "core-image-minimal".into(),
        machine: Some("qemux86-64".into()),
        source: Some("/workspace/yocto".into()),
        build_dir: Some("/workspace/yocto/build".into()),
        outcome: SavedBuildOutcome::Failed,
        saved_unix_ms: now - 120_000,
        started_unix_ms: Some(now - 1_098_000),
        finished_unix_ms: Some(now - 120_000),
        logs: vec![
            SavedBuildLog {
                unix_ms: now - 130_000,
                severity: Severity::Info,
                message: "NOTE: Running task busybox:do_compile".into(),
                recipe: None,
                task: None,
                path: None,
                build: None,
            },
            SavedBuildLog {
                unix_ms: now - 120_000,
                severity: Severity::Error,
                message: "ERROR: busybox do_compile: compiler reported a missing header".into(),
                recipe: Some("busybox".into()),
                task: Some("do_compile".into()),
                path: Some("/tmp/work/busybox/temp/log.do_compile".into()),
                build: Some("core-image-minimal".into()),
            },
        ],
        tasks: vec![SavedBuildTask {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            status: "Failed".into(),
        }],
        limitations: vec!["Bounded saved excerpt; complete logs may be unavailable.".into()],
    };
    let first = record.clone();
    record.id = "fixture-build-2".into();
    record.target = "core-image-base".into();
    record.outcome = SavedBuildOutcome::Succeeded;
    record.logs.clear();
    record.tasks.clear();
    record.saved_unix_ms = now - 7_200_000;
    app.saved_builds.records = std::sync::Arc::new(vec![first, record]);
    app.saved_builds.browsing = true;
    if scene != "offline-dashboard" {
        app.screen = Screen::BuildHistory;
        app.focus = FocusTarget::Workspace;
        app.navigator_selection = 0;
        if scene == "saved-build-logs" {
            app.saved_builds.view = Some(SavedBuildView::Logs);
        }
    }
    app
}
