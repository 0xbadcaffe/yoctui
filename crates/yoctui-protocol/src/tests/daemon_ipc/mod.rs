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

mod daemon_ipc_retries_interrupted_reads_without_losing_bytes;

mod daemon_ipc_binds_private_socket_and_authenticates_peer;

mod daemon_ipc_sends_one_preencoded_frame_without_reserialization;

mod daemon_ipc_readiness_is_nonblocking_and_observes_peer_input;

mod listener_readiness_wakes_promptly_before_a_long_deadline;

mod server_send_ignores_peer_disconnect_without_poisoning_daemon;

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

mod daemon_ipc_event_backpressure_resumes_exact_bytes_without_interleaving;

mod daemon_ipc_stalled_event_expires_and_peer_close_is_reported;

mod daemon_listener_wakes_for_pending_output_without_peer_input;

mod server_write_timeout_poisoning_prevents_a_followup_frame;

mod security_daemon_enforces_private_runtime_and_same_uid_peer;

mod daemon_ipc_removes_only_owned_stale_socket_and_reconnects;

mod daemon_ipc_rejects_unsafe_paths_and_reports_unavailable_timeout;

mod daemon_ipc_read_timeout_and_message_bound_are_typed;

mod daemon_ipc_retains_partial_frame_across_read_timeout;

mod daemon_ipc_configures_read_and_write_deadlines_independently;

mod daemon_listener_wait_wakes_for_attached_client_input;

mod daemon_listener_wait_wakes_for_new_connection;

mod daemon_listener_wait_wakes_for_additional_readiness_fd;

mod incremental_send_restores_the_snapshot_write_deadline;
