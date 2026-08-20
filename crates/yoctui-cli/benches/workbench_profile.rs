use std::{
    hint::black_box,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use ratatui::{Terminal, backend::TestBackend};
use yoctui_model::{
    Action, App, BuildStatus, CompletedTask, FocusTarget, HostTelemetry, Layer, LogEntry, Recipe,
    Screen, Severity, TaskId, TaskInfo, TaskState, update,
};

const WIDTH: u16 = 160;
const HEIGHT: u16 = 48;
const DEFAULT_FRAMES: usize = 6_000;
const SCENARIOS: [&str; 5] = [
    "idle",
    "active-build",
    "large-metadata",
    "log-heavy",
    "telemetry",
];

fn fixed_now() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_700_000_000)
}

fn workload() -> App {
    let mut app = App::new(4_096, 8 * 1024 * 1024);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;
    app.backend = "bridge".into();
    app.workspace.build_dir = Some("/profile/yocto/build".into());
    app.workspace.source_dir = Some("/profile/yocto".into());
    app.workspace.release = Some("profile-release".into());
    app.workspace.bitbake_version = Some("profile-bitbake".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    app.workspace.layers = (0..24)
        .map(|index| Layer {
            name: format!("meta-profile-{index:02}"),
            path: format!("/profile/yocto/meta-profile-{index:02}").into(),
            priority: Some(5 + index),
        })
        .collect();
    app.workspace.recipes = (0..512)
        .map(|index| Recipe {
            name: format!("profile-recipe-{index:04}"),
            version: Some(format!("1.{index}")),
            layer: Some(format!("meta-profile-{:02}", index % 24)),
            ..Default::default()
        })
        .collect();
    app.available_images = vec![
        "core-image-minimal".into(),
        "core-image-full-cmdline".into(),
    ];
    app.build.status = BuildStatus::Running;
    app.build.target = Some("core-image-minimal".into());
    app.build.started = Some(fixed_now() - Duration::from_secs(900));
    app.build.completed = 2_400;
    app.build.total = Some(4_000);
    app.host_telemetry = HostTelemetry {
        cpu_utilization_percent: Some(67),
        logical_cpu_count: Some(16),
        memory_total_bytes: Some(32 * 1024 * 1024 * 1024),
        memory_available_bytes: Some(12 * 1024 * 1024 * 1024),
        disk_available_bytes: Some(180 * 1024 * 1024 * 1024),
        disk_total_bytes: Some(512 * 1024 * 1024 * 1024),
        load_average_milli: Some([2_100, 1_800, 1_500]),
        ..HostTelemetry::default()
    };

    for index in 0..256 {
        let state = match index % 4 {
            0 | 1 => TaskState::Active,
            2 => TaskState::Waiting,
            _ => TaskState::Completed,
        };
        let task = TaskInfo {
            id: TaskId(format!("profile-recipe-{index:04}:do_compile")),
            recipe: format!("profile-recipe-{index:04}"),
            task: "do_compile".into(),
            progress: (state == TaskState::Active).then_some((index % 101) as u8),
            state,
            worker: Some(format!("worker-{}", index % 16)),
            pid: Some(10_000 + index as u32),
            started: Some(fixed_now() - Duration::from_secs((index + 1) as u64)),
            ..Default::default()
        };
        if state == TaskState::Completed {
            app.completed_tasks.push_back(CompletedTask {
                task,
                success: true,
            });
        } else {
            app.tasks.insert(task.id.clone(), task);
        }
    }

    for index in 0..1_024 {
        let _ = update(
            &mut app,
            Action::Log(LogEntry {
                id: 0,
                severity: if index % 97 == 0 {
                    Severity::Warning
                } else {
                    Severity::Info
                },
                message: format!(
                    "profile task output {index:04}: compiling deterministic workbench input"
                ),
                recipe: Some(format!("profile-recipe-{:04}", index % 256)),
                task: Some("do_compile".into()),
                path: None,
                timestamp: fixed_now() - Duration::from_secs((1_024 - index) as u64),
                build: Some("core-image-minimal".into()),
                protected: false,
                diagnostic: None,
            }),
        );
    }
    app
}

fn scenario_workload(scenario: &str) -> App {
    match scenario {
        "idle" => {
            let mut app = App::new(512, 1024 * 1024);
            app.screen = Screen::Dashboard;
            app.focus = FocusTarget::Workspace;
            app.workspace.build_dir = Some("/profile/yocto/build".into());
            app
        }
        "active-build" => workload(),
        "large-metadata" => {
            let mut app = workload();
            app.screen = Screen::Recipes;
            app.tasks.clear();
            app.completed_tasks.clear();
            app.logs.entries.clear();
            app.build.status = BuildStatus::Idle;
            app.workspace.layers = (0..1_024)
                .map(|index| Layer {
                    name: format!("meta-profile-{index:04}"),
                    path: format!("/profile/yocto/meta-profile-{index:04}").into(),
                    priority: Some(5 + index),
                })
                .collect();
            app.workspace.recipes = (0..4_096)
                .map(|index| Recipe {
                    name: format!("profile-recipe-{index:05}"),
                    version: Some(format!("1.{index}")),
                    layer: Some(format!("meta-profile-{:04}", index % 1_024)),
                    ..Default::default()
                })
                .collect();
            app
        }
        "log-heavy" => {
            let mut app = workload();
            app.screen = Screen::Logs;
            for index in 1_024..4_096 {
                let _ = update(
                    &mut app,
                    Action::Log(LogEntry {
                        id: 0,
                        severity: if index % 251 == 0 {
                            Severity::Error
                        } else if index % 97 == 0 {
                            Severity::Warning
                        } else {
                            Severity::Info
                        },
                        message: format!(
                            "retained log-heavy output {index:04}: bounded deterministic text"
                        ),
                        recipe: Some(format!("profile-recipe-{:04}", index % 256)),
                        task: Some("do_compile".into()),
                        path: None,
                        timestamp: fixed_now() - Duration::from_secs((4_096 - index) as u64),
                        build: Some("core-image-minimal".into()),
                        protected: index % 251 == 0,
                        diagnostic: None,
                    }),
                );
            }
            app
        }
        "telemetry" => {
            let mut app = workload();
            for index in 0..yoctui_model::HOST_TELEMETRY_HISTORY_SAMPLES {
                let telemetry = HostTelemetry {
                    cpu_utilization_percent: Some((index * 7 % 101) as u8),
                    logical_cpu_count: Some(16),
                    memory_total_bytes: Some(32 * 1024 * 1024 * 1024),
                    memory_available_bytes: Some((8 + index as u64 % 16) * 1024 * 1024 * 1024),
                    disk_read_bytes_per_second: Some(1_000_000 + index as u64 * 31_337),
                    disk_write_bytes_per_second: Some(500_000 + index as u64 * 17_171),
                    network_receive_bytes_per_second: Some(100_000 + index as u64 * 7_777),
                    network_transmit_bytes_per_second: Some(80_000 + index as u64 * 5_555),
                    ..HostTelemetry::default()
                };
                app.host_telemetry_history.record(&telemetry);
                app.host_telemetry = telemetry;
            }
            app
        }
        _ => panic!("unknown profile scenario {scenario:?}; expected one of {SCENARIOS:?}"),
    }
}

fn checksum(terminal: &Terminal<TestBackend>) -> u64 {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .flat_map(|cell| cell.symbol().as_bytes())
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn main() {
    let frames = std::env::var("YOCTUI_PROFILE_FRAMES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|frames| *frames > 0)
        .unwrap_or(DEFAULT_FRAMES);
    let scenario =
        std::env::var("YOCTUI_PROFILE_SCENARIO").unwrap_or_else(|_| "active-build".into());
    let mut app = scenario_workload(&scenario);
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT))
        .expect("profile terminal must be constructible");
    let started = Instant::now();
    for frame_index in 0..frames {
        if scenario == "large-metadata" {
            app.screen = if frame_index.is_multiple_of(2) {
                Screen::Recipes
            } else {
                Screen::Layers
            };
        }
        let _ = update(&mut app, Action::Tick);
        terminal
            .draw(|frame| yoctui_ui::render_at(frame, &app, fixed_now()))
            .expect("profile frame must render");
        black_box(app.animation_frame);
    }
    let checksum = black_box(checksum(&terminal));
    let elapsed = started.elapsed();
    let elapsed_ms = elapsed.as_millis();
    println!(
        "yoctui workbench profile: frames={frames} checksum={checksum:016x} elapsed_ms={elapsed_ms}"
    );
    println!(
        "yoctui ui performance: scenario={scenario} frames={frames} checksum={checksum:016x} elapsed_ms={elapsed_ms} ns_per_frame={}",
        elapsed.as_nanos() / frames as u128
    );
}
