//! Secure local Unix-domain transport for the daemon protocol.
use crate::daemon::{DaemonProtocolError, MAX_FRAME_BYTES, decode_frame, encode_frame};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    env, fs,
    io::{self, Read, Write},
    os::unix::{
        fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt},
        io::{AsRawFd, RawFd},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;

pub const RUNTIME_DIRECTORY_MODE: u32 = 0o700;
pub const SOCKET_MODE: u32 = 0o600;
const CONNECT_RETRY_INTERVAL: Duration = Duration::from_millis(10);

include!("daemon_ipc/runtime_listener.rs");
include!("daemon_ipc/connection.rs");
include!("daemon_ipc/path_security.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::ClientMessage;
    use std::sync::mpsc;

    struct InterruptOnce<R> {
        inner: R,
        interrupted: bool,
    }

    impl<R: Read> Read for InterruptOnce<R> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if !self.interrupted {
                self.interrupted = true;
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            self.inner.read(buffer)
        }
    }

    fn test_paths(name: &str) -> RuntimePaths {
        let root = env::temp_dir().join(format!(
            "yoctui-daemon-ipc-{name}-{}-{}",
            std::process::id(),
            Instant::now().elapsed().as_nanos()
        ));
        fs::DirBuilder::new()
            .mode(RUNTIME_DIRECTORY_MODE)
            .create(&root)
            .unwrap();
        runtime_paths_for(root, effective_uid()).unwrap()
    }

    fn cleanup(paths: &RuntimePaths) {
        let _ = fs::remove_file(&paths.socket);
        let _ = fs::remove_dir(&paths.directory);
        if let Some(root) = paths.directory.parent() {
            let _ = fs::remove_dir(root);
        }
    }

    #[test]
    fn daemon_ipc_retries_interrupted_reads_without_losing_bytes() {
        let mut reader = InterruptOnce {
            inner: io::Cursor::new(b"frame"),
            interrupted: false,
        };
        let mut buffer = [0_u8; 5];

        assert_eq!(
            read_retrying_interrupts(&mut reader, &mut buffer).unwrap(),
            buffer.len()
        );
        assert_eq!(&buffer, b"frame");
    }

    #[test]
    fn daemon_ipc_binds_private_socket_and_authenticates_peer() {
        let paths = test_paths("round-trip");
        let listener = DaemonListener::bind(&paths).unwrap();
        let metadata = fs::symlink_metadata(listener.socket_path()).unwrap();
        assert!(metadata.file_type().is_socket());
        assert_eq!(metadata.permissions().mode() & 0o777, SOCKET_MODE);
        assert_eq!(metadata.uid(), effective_uid());

        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            let mut client =
                DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
            client.set_timeout(Some(Duration::from_secs(1))).unwrap();
            client.send(&ClientMessage::Pong { nonce: 19 }).unwrap();
            client.peer_uid().unwrap()
        });
        let mut server = listener.accept(Duration::from_secs(1)).unwrap();
        server.set_timeout(Some(Duration::from_secs(1))).unwrap();
        assert_eq!(
            server.receive::<ClientMessage>().unwrap(),
            ClientMessage::Pong { nonce: 19 }
        );
        assert_eq!(client.join().unwrap(), effective_uid());
        drop(listener);
        assert!(!paths.socket.exists());
        cleanup(&paths);
    }

    #[test]
    fn daemon_ipc_sends_one_preencoded_frame_without_reserialization() {
        let paths = test_paths("preencoded-round-trip");
        let listener = DaemonListener::bind(&paths).unwrap();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            let mut client =
                DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
            client.set_timeout(Some(Duration::from_secs(1))).unwrap();
            client.receive::<ClientMessage>().unwrap()
        });
        let mut server = listener.accept(Duration::from_secs(1)).unwrap();
        server.set_timeout(Some(Duration::from_secs(1))).unwrap();
        let frame = encode_frame(&ClientMessage::Pong { nonce: 23 }).unwrap();
        server.send_encoded_frame(&frame).unwrap();
        assert_eq!(client.join().unwrap(), ClientMessage::Pong { nonce: 23 });
        assert!(matches!(
            server.send_encoded_frame(&[0, 0, 0, 2, b'{']),
            Err(IpcError::Protocol(DaemonProtocolError::InvalidLength))
        ));
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn daemon_ipc_readiness_is_nonblocking_and_observes_peer_input() {
        let (stream, mut peer) = UnixStream::pair().unwrap();
        let connection = DaemonConnection {
            stream,
            server_mode: true,
            pending: Vec::new(),
            expected_frame_len: None,
            write_poisoned: false,
            outgoing: None,
        };
        assert!(!connection.is_readable().unwrap());
        peer.write_all(&encode_frame(&ClientMessage::Pong { nonce: 31 }).unwrap())
            .unwrap();
        assert!(connection.is_readable().unwrap());
    }

    #[test]
    fn listener_readiness_wakes_promptly_before_a_long_deadline() {
        let paths = test_paths("listener-readiness");
        let listener = DaemonListener::bind(&paths).unwrap();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap()
        });

        let started = Instant::now();
        let _server = listener.accept(Duration::from_secs(2)).unwrap();
        assert!(started.elapsed() < Duration::from_millis(250));

        let _client = client.join().unwrap();
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn server_send_ignores_peer_disconnect_without_poisoning_daemon() {
        let paths = test_paths("peer-disconnect");
        let listener = DaemonListener::bind(&paths).unwrap();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            let mut connection =
                DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
            connection.send(&ClientMessage::Pong { nonce: 1 }).unwrap();
            connection
        });
        let mut server = listener.accept(Duration::from_secs(1)).unwrap();
        server.receive::<ClientMessage>().unwrap();
        drop(client.join().unwrap());
        assert!(server.send(&ClientMessage::Pong { nonce: 7 }).is_ok());
        cleanup(&paths);
    }

    fn event_pair() -> (DaemonConnection, UnixStream) {
        let (stream, peer) = UnixStream::pair().unwrap();
        (
            DaemonConnection {
                stream,
                server_mode: true,
                pending: Vec::new(),
                expected_frame_len: None,
                write_poisoned: false,
                outgoing: None,
            },
            peer,
        )
    }

    #[test]
    fn daemon_ipc_event_backpressure_resumes_exact_bytes_without_interleaving() {
        let paths = test_paths("event-backpressure");
        let listener = DaemonListener::bind(&paths).unwrap();
        let (mut server, mut peer) = event_pair();
        let frame = encode_frame(&"x".repeat(1024 * 1024)).unwrap();
        server.queue_event_frame(&frame).unwrap();
        // Deliberately leave the peer unread until its socket is full.
        for _ in 0..32 {
            assert!(!server.flush_event_frame().unwrap());
        }
        assert!(server.event_write_pending());
        server.pending = encode_frame(&ClientMessage::Pong { nonce: 3 }).unwrap();
        assert!(
            !listener
                .wait_for_activity(&[&server], Duration::from_millis(10))
                .unwrap()
        );
        assert!(server.send(&ClientMessage::Pong { nonce: 1 }).is_err());
        assert!(server.queue_event_frame(&frame).is_err());
        let expected = frame.clone();
        let reader = thread::spawn(move || {
            let mut received = vec![0; expected.len()];
            peer.read_exact(&mut received).unwrap();
            assert_eq!(received, expected);
            let next = encode_frame(&ClientMessage::Pong { nonce: 2 }).unwrap();
            let mut received = vec![0; next.len()];
            peer.read_exact(&mut received).unwrap();
            assert_eq!(received, next);
        });
        while !server.flush_event_frame().unwrap() {
            thread::yield_now();
        }
        server.send(&ClientMessage::Pong { nonce: 2 }).unwrap();
        reader.join().unwrap();
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn daemon_ipc_stalled_event_expires_and_peer_close_is_reported() {
        let (mut server, peer) = event_pair();
        let frame = encode_frame(&ClientMessage::Pong { nonce: 1 }).unwrap();
        server.queue_event_frame(&frame).unwrap();
        server.outgoing.as_mut().unwrap().2 = Instant::now() - Duration::from_secs(6);
        assert!(matches!(
            server.flush_event_frame(),
            Err(IpcError::Timeout(_))
        ));
        assert!(matches!(server.send(&1), Err(IpcError::Timeout(_))));
        drop(peer);
        let (mut server, peer) = event_pair();
        server.queue_event_frame(&frame).unwrap();
        drop(peer);
        assert!(server.flush_event_frame().is_err());
        assert!(server.write_poisoned);
        assert!(server.queue_event_frame(&frame).is_err());
    }

    #[test]
    fn daemon_listener_wakes_for_pending_output_without_peer_input() {
        let paths = test_paths("pending-output");
        let listener = DaemonListener::bind(&paths).unwrap();
        let (mut server, _peer) = event_pair();
        server
            .queue_event_frame(&encode_frame(&42).unwrap())
            .unwrap();
        let started = Instant::now();
        assert!(
            listener
                .wait_for_activity(&[&server], Duration::from_secs(2))
                .unwrap()
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(server.flush_event_frame().unwrap());
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn server_write_timeout_poisoning_prevents_a_followup_frame() {
        let paths = test_paths("write-timeout");
        let listener = DaemonListener::bind(&paths).unwrap();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            let connection =
                DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
            thread::sleep(Duration::from_millis(200));
            connection
        });
        let mut server = listener.accept(Duration::from_secs(1)).unwrap();
        server
            .set_write_timeout(Some(Duration::from_millis(20)))
            .unwrap();

        server.send(&"x".repeat(3 * 1024 * 1024)).unwrap();
        assert!(server.write_poisoned);
        assert!(matches!(
            server.send(&ClientMessage::Pong { nonce: 31 }),
            Err(IpcError::Disconnected)
        ));

        drop(client.join().unwrap());
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn security_daemon_enforces_private_runtime_and_same_uid_peer() {
        let paths = test_paths("security");
        let listener = DaemonListener::bind(&paths).unwrap();
        let metadata = fs::symlink_metadata(listener.socket_path()).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, SOCKET_MODE);
        assert_eq!(metadata.uid(), effective_uid());
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            let client = DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
            client.peer_uid().unwrap()
        });
        let server = listener.accept(Duration::from_secs(1)).unwrap();
        assert_eq!(server.peer_uid().unwrap(), effective_uid());
        assert_eq!(client.join().unwrap(), effective_uid());
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn daemon_ipc_removes_only_owned_stale_socket_and_reconnects() {
        let paths = test_paths("stale");
        fs::create_dir(&paths.directory).unwrap();
        fs::set_permissions(
            &paths.directory,
            fs::Permissions::from_mode(RUNTIME_DIRECTORY_MODE),
        )
        .unwrap();
        let stale = UnixListener::bind(&paths.socket).unwrap();
        drop(stale);
        let listener = DaemonListener::bind(&paths).unwrap();

        let (started_tx, started_rx) = mpsc::channel();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            started_tx.send(()).unwrap();
            DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap()
        });
        started_rx.recv().unwrap();
        let _server = listener.accept(Duration::from_secs(1)).unwrap();
        let _client = client.join().unwrap();
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn daemon_ipc_rejects_unsafe_paths_and_reports_unavailable_timeout() {
        assert!(matches!(
            runtime_paths_for(PathBuf::from("relative"), effective_uid()),
            Err(IpcError::RelativeRuntimePath(_))
        ));
        let paths = test_paths("unsafe");
        fs::create_dir(&paths.directory).unwrap();
        fs::set_permissions(&paths.directory, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(
            DaemonListener::bind(&paths),
            Err(IpcError::UnsafeRuntimePath { .. })
        ));
        fs::set_permissions(
            &paths.directory,
            fs::Permissions::from_mode(RUNTIME_DIRECTORY_MODE),
        )
        .unwrap();
        fs::write(&paths.socket, b"not a socket").unwrap();
        assert!(matches!(
            DaemonListener::bind(&paths),
            Err(IpcError::UnsafeRuntimePath { .. })
        ));
        fs::remove_file(&paths.socket).unwrap();
        std::os::unix::fs::symlink("/tmp", &paths.socket).unwrap();
        assert!(matches!(
            DaemonListener::bind(&paths),
            Err(IpcError::UnsafeRuntimePath { .. })
        ));
        fs::remove_file(&paths.socket).unwrap();
        assert!(matches!(
            DaemonConnection::connect(&paths, Duration::from_millis(20)),
            Err(IpcError::Unavailable { .. })
        ));
        cleanup(&paths);
    }

    #[test]
    fn daemon_ipc_read_timeout_and_message_bound_are_typed() {
        let paths = test_paths("limits");
        let listener = DaemonListener::bind(&paths).unwrap();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            let mut connection =
                DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
            connection
                .stream
                .write_all(&((MAX_FRAME_BYTES as u32 + 1).to_be_bytes()))
                .unwrap();
        });
        let mut server = listener.accept(Duration::from_secs(1)).unwrap();
        server.set_timeout(Some(Duration::from_millis(50))).unwrap();
        assert!(matches!(
            server.receive::<ClientMessage>(),
            Err(IpcError::Protocol(DaemonProtocolError::TooLarge))
        ));
        client.join().unwrap();

        let waiting_paths = paths.clone();
        let waiting_client = thread::spawn(move || {
            DaemonConnection::connect(&waiting_paths, Duration::from_secs(1)).unwrap()
        });
        let mut waiting_server = listener.accept(Duration::from_secs(1)).unwrap();
        waiting_server
            .set_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        assert!(matches!(
            waiting_server.receive::<ClientMessage>(),
            Err(IpcError::Timeout(_))
        ));
        drop(waiting_server);
        let _ = waiting_client.join().unwrap();
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn daemon_ipc_retains_partial_frame_across_read_timeout() {
        let paths = test_paths("partial-timeout");
        let listener = DaemonListener::bind(&paths).unwrap();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            let connection =
                DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
            let frame = encode_frame(&ClientMessage::Pong { nonce: 29 }).unwrap();
            (&connection.stream).write_all(&frame[..8]).unwrap();
            thread::sleep(Duration::from_millis(60));
            (&connection.stream).write_all(&frame[8..]).unwrap();
        });
        let mut server = listener.accept(Duration::from_secs(1)).unwrap();
        server
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();

        assert!(matches!(
            server.receive::<ClientMessage>(),
            Err(IpcError::Timeout(_))
        ));
        client.join().unwrap();
        assert_eq!(
            server.receive::<ClientMessage>().unwrap(),
            ClientMessage::Pong { nonce: 29 }
        );

        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn daemon_ipc_configures_read_and_write_deadlines_independently() {
        let paths = test_paths("independent-timeouts");
        let listener = DaemonListener::bind(&paths).unwrap();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap()
        });
        let server = listener.accept(Duration::from_secs(1)).unwrap();
        let read_timeout = Duration::from_millis(20);
        let write_timeout = Duration::from_secs(2);

        server.set_read_timeout(Some(read_timeout)).unwrap();
        server.set_write_timeout(Some(write_timeout)).unwrap();

        assert_eq!(server.stream.read_timeout().unwrap(), Some(read_timeout));
        assert_eq!(server.stream.write_timeout().unwrap(), Some(write_timeout));
        drop(client.join().unwrap());
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn daemon_listener_wait_wakes_for_attached_client_input() {
        let paths = test_paths("activity-client");
        let listener = DaemonListener::bind(&paths).unwrap();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            let mut connection =
                DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
            thread::sleep(Duration::from_millis(20));
            connection.send(&ClientMessage::Pong { nonce: 41 }).unwrap();
        });
        let mut server = listener.accept(Duration::from_secs(1)).unwrap();
        let started = Instant::now();

        assert!(
            listener
                .wait_for_activity(&[&server], Duration::from_secs(1))
                .unwrap()
        );
        assert!(started.elapsed() < Duration::from_millis(500));
        assert_eq!(
            server.receive::<ClientMessage>().unwrap(),
            ClientMessage::Pong { nonce: 41 }
        );

        client.join().unwrap();
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn daemon_listener_wait_wakes_for_new_connection() {
        let paths = test_paths("activity-listener");
        let listener = DaemonListener::bind(&paths).unwrap();
        let client_paths = paths.clone();
        let client = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap()
        });
        let started = Instant::now();

        assert!(
            listener
                .wait_for_activity(&[], Duration::from_secs(1))
                .unwrap()
        );
        assert!(started.elapsed() < Duration::from_millis(500));
        let _server = listener.accept(Duration::ZERO).unwrap();

        drop(client.join().unwrap());
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn daemon_listener_wait_wakes_for_additional_readiness_fd() {
        let paths = test_paths("activity-additional-fd");
        let listener = DaemonListener::bind(&paths).unwrap();
        let (mut writer, reader) = UnixStream::pair().unwrap();
        let notifier = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            writer.write_all(&[1]).unwrap();
        });
        let started = Instant::now();

        assert!(
            listener
                .wait_for_activity_with_additional_fd(
                    &[],
                    Some(reader.as_raw_fd()),
                    Duration::from_secs(1),
                )
                .unwrap()
        );
        assert!(started.elapsed() < Duration::from_millis(500));

        notifier.join().unwrap();
        drop(listener);
        cleanup(&paths);
    }

    #[test]
    fn incremental_send_restores_the_snapshot_write_deadline() {
        let (stream, mut peer) = UnixStream::pair().unwrap();
        let mut connection = DaemonConnection {
            stream,
            server_mode: true,
            pending: Vec::new(),
            expected_frame_len: None,
            write_poisoned: false,
            outgoing: None,
        };
        let snapshot_deadline = Duration::from_secs(1);
        connection
            .set_write_timeout(Some(snapshot_deadline))
            .unwrap();
        let frame = encode_frame(&ClientMessage::Pong { nonce: 37 }).unwrap();

        connection
            .send_encoded_frame_with_timeout(&frame, Duration::from_millis(2))
            .unwrap();

        assert_eq!(
            connection.stream.write_timeout().unwrap(),
            Some(snapshot_deadline)
        );
        let mut received = vec![0; frame.len()];
        peer.read_exact(&mut received).unwrap();
        assert_eq!(received, frame);
    }
}
