#![cfg(unix)]

use std::{
    io::{Read, Write},
    os::unix::net::UnixListener,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn round_trip(explicit_socket: bool) {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("relay.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_yoctui"));
    command.arg("__menuconfig-handoff");
    if explicit_socket {
        command.arg("--socket").arg(&socket);
        command.env_remove("YOCTUI_MENUCONFIG_SOCKET");
    } else {
        command.env("YOCTUI_MENUCONFIG_SOCKET", &socket);
    }
    let mut child = command
        .args(["--", "/helper", "/pidfile", "/wrapper"])
        .current_dir(root.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline || child.try_wait().unwrap().is_some() {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("handoff did not reach its request-specific relay");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("{error}"),
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut count = [0; 4];
    stream.read_exact(&mut count).unwrap();
    assert_eq!(u32::from_be_bytes(count), 4);
    let mut arguments = Vec::new();
    for _ in 0..4 {
        stream.read_exact(&mut count).unwrap();
        let length = u32::from_be_bytes(count) as usize;
        assert!((1..=16 * 1024).contains(&length));
        let mut bytes = vec![0; length];
        stream.read_exact(&mut bytes).unwrap();
        arguments.push(String::from_utf8(bytes).unwrap());
    }
    assert_eq!(Path::new(&arguments[0]), root.path());
    assert_eq!(&arguments[1..], ["/helper", "/pidfile", "/wrapper"]);
    stream.write_all(&[0]).unwrap();
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("handoff did not acknowledge completion");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(child.wait().unwrap().success());
}

#[test]
fn menuconfig_handoff_gets_its_unique_socket_from_original_environment() {
    round_trip(false);
}

#[test]
fn menuconfig_handoff_still_accepts_legacy_explicit_socket_argument() {
    round_trip(true);
}

#[test]
fn menuconfig_handoff_rejects_missing_or_relative_socket_without_waiting() {
    for socket in [None, Some("relative.sock")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_yoctui"));
        command
            .arg("__menuconfig-handoff")
            .env_remove("YOCTUI_MENUCONFIG_SOCKET");
        if let Some(socket) = socket {
            command.args(["--socket", socket]);
        }
        let result = command
            .args(["--", "/helper", "/pidfile", "/wrapper"])
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("socket"));
    }
}
