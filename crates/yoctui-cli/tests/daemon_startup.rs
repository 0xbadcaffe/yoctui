#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    process::Command,
    time::{Duration, Instant},
};

#[test]
fn daemon_startup_serves_ipc_and_stops_during_slow_recipe_inventory() {
    check_startup_inventory(false);
}

#[test]
fn daemon_startup_publishes_completed_inventory_without_reattachment() {
    check_startup_inventory(true);
}

fn check_startup_inventory(complete: bool) {
    let root = std::env::temp_dir().join(format!(
        "yoctui-slow-startup-{}-{complete}",
        std::process::id()
    ));
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    let build = root.join("build");
    let bin = root.join("bin");
    fs::create_dir_all(build.join("conf")).unwrap();
    fs::create_dir(&bin).unwrap();
    fs::write(build.join("conf/local.conf"), "MACHINE = \"qemux86-64\"\n").unwrap();
    fs::write(build.join("conf/bblayers.conf"), "BBLAYERS = \"\"\n").unwrap();
    fs::write(
        bin.join("bitbake"),
        "#!/bin/sh\necho 'BitBake Build Tool Core version 2.18.0'\n",
    )
    .unwrap();
    fs::set_permissions(bin.join("bitbake"), fs::Permissions::from_mode(0o700)).unwrap();
    let bridge = root.join("bridge.py");
    let fixture = include_str!("../../../scripts/fixtures/bitbake-ipc-latency-bridge.py")
        .replace("import os\n", "import os\nimport signal\nsignal.signal(signal.SIGINT, signal.default_int_handler)\n")
        .replace("emit({\"type\": \"recipes\", \"recipes\": []}, correlation)",
            "open('scan-started', 'w').close()\n        try:\n            time.sleep(60)\n        finally:\n            open('scan-cleaned', 'w').close()\n        emit({\"type\": \"recipes\", \"recipes\": []}, correlation)");
    assert!(fixture.contains("time.sleep(60)"));
    fs::write(
        &bridge,
        if complete {
            fixture.replace(
                "time.sleep(60)",
                "while not os.path.exists('finish-scan'):\n                time.sleep(0.02)",
            )
        } else {
            fixture
        },
    )
    .unwrap();
    let run = |action: &str| {
        Command::new(env!("CARGO_BIN_EXE_yoctui"))
            .args(["daemon", action])
            .env("XDG_RUNTIME_DIR", &root)
            .env("XDG_STATE_HOME", root.join("state"))
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("YOCTUI_DAEMON_LOG", root.join("daemon.log"))
            .env("BUILDDIR", &build)
            .env("PYTHON", "/usr/bin/python3")
            .env("PATH", &bin)
            .env("YOCTUI_BRIDGE_PATH", &bridge)
            .output()
            .unwrap()
    };
    let started = Instant::now();
    let start = run("start");
    let start_elapsed = started.elapsed();
    // Always stop the fixture before asserting, including regression failures.
    let deadline = Instant::now() + Duration::from_secs(5);
    while !build.join("scan-started").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let status = run("status");
    let delivered = if complete {
        use yoctui_protocol::{
            daemon::*,
            daemon_ipc::{DaemonConnection, runtime_paths_for},
        };
        let paths = runtime_paths_for(root.clone(), unsafe { libc::geteuid() }).unwrap();
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
        let _ = connection.receive::<ServerMessage>().unwrap();
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
        let _ = connection.receive::<ServerMessage>().unwrap();
        fs::write(build.join("finish-scan"), "").unwrap();
        let mut received = false;
        let deadline = Instant::now() + Duration::from_secs(5);
        while !received && Instant::now() < deadline {
            match connection.receive::<ServerMessage>() {
                Ok(ServerMessage::Event(event)) => {
                    received = matches!(event.event,
                    DaemonEvent::Build(DaemonBuildEvent::Workspace { data }) if data.build_dir.as_deref() == build.to_str())
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        received
    } else {
        true
    };
    let stopped = Instant::now();
    let stop = run("stop");
    assert!(start.status.success(), "{:?}", start);
    assert!(start_elapsed < Duration::from_secs(10), "{start_elapsed:?}");
    assert!(
        build.join("scan-started").exists(),
        "{}",
        fs::read_to_string(root.join("daemon.log")).unwrap()
    );
    assert!(status.status.success(), "{:?}", status);
    assert!(stop.status.success(), "{:?}", stop);
    assert!(
        delivered,
        "completed inventory was not published to the attached client"
    );
    assert!(stopped.elapsed() < Duration::from_secs(10));
    assert!(build.join("scan-cleaned").exists());
    assert!(!root.join("yoctui/daemon.sock").exists());
    assert!(!root.join("yoctui/daemon.json").exists());
    fs::remove_dir_all(root).unwrap();
}
