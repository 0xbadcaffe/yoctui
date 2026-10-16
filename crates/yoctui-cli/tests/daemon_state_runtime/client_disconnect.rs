use super::*;
use std::{io::Write, net::Shutdown, os::unix::net::UnixStream, time::Instant};
use yoctui_protocol::daemon::encode_frame;

#[test]
fn client_disconnect_between_replies_preserves_the_running_daemon() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_yoctui"));
    let runtime = unique_temp_root("yoctui-client-disconnect");
    fs::DirBuilder::new().mode(0o700).create(&runtime).unwrap();
    let _guard = DaemonGuard {
        binary: binary.clone(),
        runtime: runtime.clone(),
    };
    let start = Command::new(&binary)
        .args(["daemon", "start"])
        .env("XDG_RUNTIME_DIR", &runtime)
        .env("XDG_STATE_HOME", runtime.join("state"))
        .env("XDG_CONFIG_HOME", runtime.join("config"))
        .env("XDG_DATA_HOME", runtime.join("data"))
        .env_remove("BUILDDIR")
        .output()
        .unwrap();
    assert!(start.status.success(), "{start:?}");
    let (before, _) = attach_snapshot(&runtime, None);
    let paths = runtime_paths_for(runtime.clone(), unsafe { libc::geteuid() }).unwrap();
    let mut broken = UnixStream::connect(&paths.socket).unwrap();
    broken
        .set_write_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    // Reject replies before sending multiple messages. The first failed write
    // poisons the server-side stream; a second reply must retire only this peer.
    broken.shutdown(Shutdown::Read).unwrap();
    let hello = encode_frame(&ClientMessage::Hello(ClientHello {
        minimum_version: ProtocolVersion::CURRENT,
        maximum_version: ProtocolVersion::CURRENT,
        client_id: ClientId([17; 16]),
        client_name: "disconnected-before-replies".into(),
        capabilities: vec![Capability::StateSnapshots],
    }))
    .unwrap();
    let mut burst = hello.clone();
    burst.extend_from_slice(&hello);
    broken.write_all(&burst).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while broken.write_all(&hello).is_ok() {
        assert!(Instant::now() < deadline, "broken client was not retired");
        std::thread::sleep(Duration::from_millis(10));
    }
    let (after, _) = attach_snapshot(&runtime, None);
    assert_eq!(after.daemon_instance_id, before.daemon_instance_id);
}
