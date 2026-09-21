use super::*;

#[test]
fn resource_limits_are_explicit_and_bounded() {
    const {
        assert!(MAX_DAEMON_CLIENTS < u16::MAX as usize);
        assert!(MAX_DAEMON_PTY_SESSIONS < u16::MAX as usize);
        assert!(MAX_TERMINAL_SCROLLBACK_LINES <= MAX_SNAPSHOT_LOGS);
        assert!(MAX_PTY_OUTPUT_EVENT_BYTES <= MAX_FRAME_BYTES);
        assert!(MAX_UTILITY_OUTPUT_BYTES <= MAX_FRAME_BYTES);
    }
    let limits = ProtocolLimits {
        maximum_frame_bytes: MAX_FRAME_BYTES as u32,
        maximum_snapshot_bytes: MAX_FRAME_BYTES as u32,
        maximum_pending_requests: 64,
        maximum_queue_depth: 256,
        maximum_terminal_rows: 512,
        maximum_terminal_columns: 512,
        maximum_clients: MAX_DAEMON_CLIENTS as u16,
        maximum_pty_sessions: MAX_DAEMON_PTY_SESSIONS as u16,
        maximum_scrollback_lines: MAX_TERMINAL_SCROLLBACK_LINES as u32,
        maximum_utility_output_bytes: MAX_UTILITY_OUTPUT_BYTES as u32,
    };
    let round_trip: ProtocolLimits =
        serde_json::from_slice(&serde_json::to_vec(&limits).unwrap()).unwrap();
    assert_eq!(round_trip, limits);
}
