#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    process::{Command, Output},
    time::{Duration, Instant},
};
use yoctui_protocol::{
    daemon::*,
    daemon_ipc::{DaemonConnection, runtime_paths_for},
};

struct Fixture {
    root: PathBuf,
    build: PathBuf,
    bin: PathBuf,
}

impl Fixture {
    fn new(name: &str, bitbake: &str, initialized: bool) -> Self {
        let root =
            std::env::temp_dir().join(format!("yoctui-startup-{}-{name}", std::process::id()));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let build = root.join("build");
        let bin = root.join("bin");
        fs::create_dir_all(build.join("conf")).unwrap();
        fs::create_dir(&bin).unwrap();
        fs::write(build.join("conf/local.conf"), "MACHINE = \"qemux86-64\"\n").unwrap();
        if initialized {
            fs::write(build.join("conf/bblayers.conf"), "BBLAYERS = \"\"\n").unwrap();
        }
        fs::write(bin.join("bitbake"), bitbake).unwrap();
        fs::set_permissions(bin.join("bitbake"), fs::Permissions::from_mode(0o700)).unwrap();
        Self { root, build, bin }
    }

    fn run(&self, action: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_yoctui"))
            .args(["daemon", action])
            .env("XDG_RUNTIME_DIR", &self.root)
            .env("XDG_STATE_HOME", self.root.join("state"))
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("YOCTUI_DAEMON_LOG", self.root.join("daemon.log"))
            .env("BUILDDIR", &self.build)
            .env("PYTHON", "/usr/bin/python3")
            .env("PATH", &self.bin)
            .env("YOCTUI_BRIDGE_PATH", self.root.join("bridge.py"))
            .output()
            .unwrap()
    }

    fn attach(&self) -> (DaemonConnection, DaemonSnapshot) {
        let paths = runtime_paths_for(self.root.clone(), unsafe { libc::geteuid() }).unwrap();
        let mut connection = DaemonConnection::connect(&paths, Duration::from_secs(2)).unwrap();
        connection
            .set_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        connection
            .send(&ClientMessage::Hello(ClientHello {
                minimum_version: ProtocolVersion::CURRENT,
                maximum_version: ProtocolVersion::CURRENT,
                client_id: ClientId([73; 16]),
                client_name: "startup-inventory-test".into(),
                capabilities: vec![Capability::StateSnapshots, Capability::IncrementalEvents],
            }))
            .unwrap();
        assert!(matches!(
            connection.receive::<ServerMessage>().unwrap(),
            ServerMessage::Hello(_)
        ));
        connection
            .send(&ClientMessage::Attach {
                workspace: None,
                resume: None,
                subscription: Subscription {
                    state: true,
                    jobs: true,
                    logs: true,
                    pty_sessions: vec![],
                },
            })
            .unwrap();
        let ServerMessage::Attached { snapshot, .. } =
            connection.receive::<ServerMessage>().unwrap()
        else {
            panic!("expected attached snapshot");
        };
        (connection, snapshot)
    }

    fn wait_for(&self, ready: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(12);
        while !ready() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            ready(),
            "{}",
            fs::read_to_string(self.root.join("daemon.log")).unwrap_or_default()
        );
    }

    fn stop(&self) {
        let stopped = Instant::now();
        let output = self.run("stop");
        assert!(output.status.success(), "{output:?}");
        assert!(stopped.elapsed() < Duration::from_secs(10));
        assert!(!self.root.join("yoctui/daemon.sock").exists());
        assert!(!self.root.join("yoctui/daemon.json").exists());
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Clean up even when a regression assertion fails before normal shutdown.
        if self.root.join("yoctui/daemon.json").exists() {
            let _ = self.run("stop");
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn daemon_startup_serves_ipc_and_stops_during_slow_recipe_inventory() {
    check_startup_inventory(false, false);
}

#[test]
fn daemon_startup_publishes_completed_inventory_without_reattachment() {
    check_startup_inventory(true, false);
}

#[test]
fn daemon_startup_serves_ipc_before_slow_compatibility_probes_complete() {
    check_startup_inventory(true, true);
}

fn check_startup_inventory(complete: bool, slow_compatibility: bool) {
    let fixture = Fixture::new(
        &format!("inventory-{complete}-{slow_compatibility}"),
        if slow_compatibility {
            "#!/bin/sh\nif [ \"$1\" = --version ]; then /bin/sleep 3; fi\necho 'BitBake Build Tool Core version 2.18.0'\n"
        } else {
            "#!/bin/sh\necho 'BitBake Build Tool Core version 2.18.0'\n"
        },
        true,
    );
    let bridge = include_str!("../../../scripts/fixtures/bitbake-ipc-latency-bridge.py")
        .replace("import os\n", "import os\nimport signal\nsignal.signal(signal.SIGINT, signal.default_int_handler)\n")
        .replace("emit({\"type\": \"recipes\", \"recipes\": []}, correlation)",
            "open('scan-started', 'w').close()\n        try:\n            time.sleep(60)\n        finally:\n            open('scan-cleaned', 'w').close()\n        emit({\"type\": \"recipes\", \"recipes\": []}, correlation)");
    assert!(bridge.contains("time.sleep(60)"));
    fs::write(
        fixture.root.join("bridge.py"),
        if complete {
            bridge.replace(
                "time.sleep(60)",
                "while not os.path.exists('finish-scan'):\n                time.sleep(0.02)",
            )
        } else {
            bridge
        },
    )
    .unwrap();
    let started = Instant::now();
    let start = fixture.run("start");
    let elapsed = started.elapsed();
    assert!(start.status.success(), "{start:?}");
    let mut early = if slow_compatibility {
        let (connection, snapshot) = fixture.attach();
        assert!(
            snapshot.compatibility.is_none(),
            "probe must still be loading"
        );
        Some(connection)
    } else {
        None
    };
    fixture.wait_for(|| fixture.build.join("scan-started").exists());
    let status = fixture.run("status");
    assert!(status.status.success(), "{status:?}");
    if complete {
        let mut connection = early.take().unwrap_or_else(|| fixture.attach().0);
        fs::write(fixture.build.join("finish-scan"), "").unwrap();
        let mut received = false;
        let mut received_compatibility = !slow_compatibility;
        let deadline = Instant::now() + Duration::from_secs(5);
        while !(received && received_compatibility) && Instant::now() < deadline {
            match connection.receive::<ServerMessage>().unwrap() {
                ServerMessage::Event(event) => match event.event {
                    DaemonEvent::CompatibilityChanged(_) => received_compatibility = true,
                    DaemonEvent::Build(DaemonBuildEvent::Workspace { data }) => {
                        assert!(
                            received_compatibility,
                            "inventory must follow compatibility"
                        );
                        received = data.build_dir.as_deref() == fixture.build.to_str();
                    }
                    _ => {}
                },
                ServerMessage::Ping { nonce, .. } => {
                    connection.send(&ClientMessage::Pong { nonce }).unwrap()
                }
                _ => {}
            }
        }
        assert!(
            received && received_compatibility,
            "startup updates not delivered to existing client"
        );
    }
    fixture.stop();
    eprintln!("daemon readiness with slow compatibility={slow_compatibility}: {elapsed:?}");
    assert!(elapsed < Duration::from_secs(if slow_compatibility { 2 } else { 10 }));
    assert!(fixture.build.join("scan-cleaned").exists());
}

#[test]
fn daemon_startup_cancels_slow_compatibility_and_reaps_probe() {
    let fixture = Fixture::new(
        "cancel-probe",
        "#!/bin/sh\necho $$ > \"$BUILDDIR/probe-pid\"\nexec /bin/sleep 60\n",
        true,
    );
    let started = Instant::now();
    let start = fixture.run("start");
    assert!(start.status.success(), "{start:?}");
    assert!(started.elapsed() < Duration::from_secs(2));
    fixture.wait_for(|| fixture.build.join("probe-pid").exists());
    assert!(fixture.attach().1.compatibility.is_none());
    let stopped = Instant::now();
    fixture.stop();
    assert!(stopped.elapsed() < Duration::from_secs(3));
    let pid: i32 = fs::read_to_string(fixture.build.join("probe-pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while unsafe { libc::kill(pid, 0) } == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "probe remains alive after shutdown"
    );
}

#[test]
fn daemon_startup_reports_failed_compatibility_and_keeps_ipc_available() {
    let fixture = Fixture::new("failed-probe", "#!/bin/sh\nexit 1\n", false);
    let start = fixture.run("start");
    assert!(start.status.success(), "{start:?}");
    fixture.wait_for(|| {
        fs::read_to_string(fixture.root.join("daemon.log"))
            .unwrap_or_default()
            .contains("Compatibility authority is unavailable")
    });
    let (_, snapshot) = fixture.attach();
    assert!(snapshot.compatibility.is_none());
    assert!(snapshot.recent_logs.iter().any(|log| {
        log.message
            .contains("Compatibility authority is unavailable")
    }));
    fixture.stop();
}
