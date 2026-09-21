//! Typed, bounded protocol for the persistent daemon and attachable clients.
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    path::{Component, Path},
};
use thiserror::Error;

use crate::{TaskStatsData, WorkspaceData};

pub const PROTOCOL_MAJOR: u16 = 1;
pub const PROTOCOL_MINOR: u16 = 3;
pub const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_CAPABILITIES: usize = 128;
pub const MAX_RETAINED_EVENTS: usize = 65_536;
pub const MAX_SNAPSHOT_LOGS: usize = 100_000;
pub const MAX_DAEMON_CLIENTS: usize = 32;
pub const MAX_DAEMON_PTY_SESSIONS: usize = 64;
pub const MAX_TERMINAL_SCROLLBACK_LINES: usize = 100_000;
pub const MAX_TERMINAL_ROWS: u16 = 512;
pub const MAX_TERMINAL_COLUMNS: u16 = 512;
pub const MAX_TERMINAL_CELLS: usize = 250_000;
pub const MAX_TERMINAL_CELL_BYTES: usize = 1_024;
pub const MAX_UTILITY_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PTY_OUTPUT_EVENT_BYTES: usize = 64 * 1024;
pub const MAX_PTY_INPUT_BYTES: usize = 64 * 1024;
pub const MAX_DAEMON_BUILD_EVENTS: usize = 2_048;
pub const COMPATIBILITY_SCHEMA_VERSION: u16 = 1;
pub const MAX_COMPATIBILITY_CAPABILITIES: usize = 512;
pub const MAX_COMPATIBILITY_EVIDENCE: usize = 32;
pub const MAX_COMPATIBILITY_ITEMS: usize = 256;
pub const MAX_COMPATIBILITY_TEXT_BYTES: usize = 4_096;
pub const MAX_COMPATIBILITY_ARGV: usize = 64;
pub const RAW_EXECUTION_SCHEMA_VERSION: u16 = 1;
pub const RAW_HISTORY_SCHEMA_VERSION: u16 = 1;
pub const MAX_RAW_HISTORY_RECORDS: usize = 256;
pub const MAX_RAW_HISTORY_AGGREGATE_BYTES: usize = 256 * 1024;
pub const MAX_RAW_EXECUTION_ID_BYTES: usize = 96;
pub const MAX_RAW_EXECUTION_REQUESTS: usize = 64;
pub const MAX_RAW_EXECUTION_PARAMETERS: usize = 32;
pub const MAX_RAW_EXECUTION_PARAMETER_ID_BYTES: usize = 96;
pub const MAX_RAW_EXECUTION_PARAMETER_BYTES: usize = 4_096;
pub const MAX_RAW_EXECUTION_ARGUMENTS: usize = 64;
pub const MAX_RAW_EXECUTION_ARGUMENT_BYTES: usize = 512;
pub const MAX_RAW_EXECUTION_ARGUMENT_AGGREGATE_BYTES: usize = 8_192;
pub const MAX_RAW_EXECUTION_BUILD_DIRECTORY_BYTES: usize = 4_096;
pub const MAX_RAW_EXECUTION_OUTPUT_CHUNK_BYTES: usize = 64 * 1_024;
pub const MAX_RAW_EXECUTION_RETAINED_BYTES: usize = 1_024 * 1_024;
pub const MAX_RAW_EXECUTION_RETAINED_LINES: usize = 10_000;
pub const MAX_RAW_EXECUTION_MESSAGE_BYTES: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self {
        major: PROTOCOL_MAJOR,
        minor: PROTOCOL_MINOR,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub [u8; 16]);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DaemonInstanceId(pub [u8; 16]);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PtySessionId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(pub u64);

include!("daemon/raw_request_history.rs");
include!("daemon/raw_events_snapshot.rs");
include!("daemon/compatibility_identity.rs");
include!("daemon/compatibility_validation.rs");

include!("daemon/client_commands.rs");
include!("daemon/qa_testing_messages.rs");
include!("daemon/terminal_messages.rs");
include!("daemon/snapshot_types.rs");
include!("daemon/snapshot_journal.rs");
include!("daemon/snapshot_reducer.rs");
include!("daemon/errors_framing.rs");

#[cfg(test)]
mod tests {
    use super::*;

    fn client_id(byte: u8) -> ClientId {
        ClientId([byte; 16])
    }

    fn daemon_snapshot_fixture() -> DaemonSnapshot {
        DaemonSnapshot {
            daemon_instance_id: DaemonInstanceId([7; 16]),
            sequence: 0,
            generation: 0,
            workspace: None,
            project_profile: ProjectProfileSummary::Absent,
            bitbake: BitBakeState {
                lifecycle: LifecycleState::Disconnected,
                version: None,
                capabilities: Vec::new(),
                diagnostic: None,
            },
            compatibility: None,
            jobs: Vec::new(),
            raw_executions: Vec::new(),
            raw_history: Vec::new(),
            pty_sessions: Vec::new(),
            pty_screens: Vec::new(),
            clients: Vec::new(),
            recent_logs: Vec::new(),
            build_events: Vec::new(),
            build_progress: None,
            recovery_warnings: Vec::new(),
        }
    }

    fn compatibility_environment_fixture() -> CompatibilityEnvironmentIdentity {
        CompatibilityEnvironmentIdentity {
            build_directory: CompatibilityDetected::Detected {
                value: "/work/poky/build".into(),
                authority: CompatibilityIdentityAuthority::InitializedEnvironment,
            },
            source_roots: CompatibilityDetected::Unknown,
            bitbake_version: CompatibilityDetected::Detected {
                value: "2.18.0".into(),
                authority: CompatibilityIdentityAuthority::BitBakeVersionProbe,
            },
            oe_core: CompatibilityDetected::Unknown,
            poky: CompatibilityDetected::Unknown,
            distro: CompatibilityDetected::Unknown,
            machine: CompatibilityDetected::Unknown,
            layer_series: CompatibilityDetected::Unknown,
            available_tools: CompatibilityDetected::Unknown,
            backend: CompatibilityDetected::Unknown,
            protocol: CompatibilityDetected::Detected {
                value: CompatibilityProtocolIdentity {
                    name: "yoctui-daemon".into(),
                    version: "1.0".into(),
                },
                authority: CompatibilityIdentityAuthority::ProtocolNegotiation,
            },
        }
    }

    fn compatibility_snapshot_fixture(generation: u64) -> CompatibilitySnapshotData {
        CompatibilitySnapshotData {
            schema_version: COMPATIBILITY_SCHEMA_VERSION,
            generation,
            environment: compatibility_environment_fixture(),
            capabilities: vec![CompatibilityCapabilityData {
                id: "bitbake.getvar".into(),
                state: CompatibilityStateData::Available,
                evidence: vec![CompatibilityEvidenceData {
                    kind: CompatibilityEvidenceKind::DirectProbe,
                    outcome: CompatibilityEvidenceOutcome::Positive,
                    subject: "bitbake --help".into(),
                    detail: "getvar option present".into(),
                    argv: vec!["bitbake".into(), "--help".into()],
                }],
                implementation: Some(CompatibilityImplementationData {
                    id: "bitbake.getvar.native".into(),
                    kind: "native".into(),
                }),
            }],
        }
    }

    #[test]
    fn compatibility_snapshot_round_trips_bounded_identity_state_and_evidence() {
        let snapshot = compatibility_snapshot_fixture(7);
        snapshot.validate().unwrap();

        let encoded = serde_json::to_vec(&snapshot).unwrap();
        let decoded: CompatibilitySnapshotData = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, snapshot);
        assert!(decoded.capabilities[0].state.is_enabled());
    }

    #[test]
    fn compatibility_snapshot_accepts_case_preserving_distro_identity() {
        let mut snapshot = compatibility_snapshot_fixture(1);
        snapshot.environment.distro = CompatibilityDetected::Detected {
            value: CompatibilityDistroIdentity {
                name: "Poky+custom".into(),
                version: Some("5.2.4".into()),
            },
            authority: CompatibilityIdentityAuthority::BitBakeDatastore,
        };
        snapshot.validate().unwrap();
    }

    #[test]
    fn nested_build_event_round_trips_without_duplicate_type_fields() {
        let event = DaemonEvent::Build(DaemonBuildEvent::Reset {
            targets: vec!["core-image-minimal".into()],
        });
        let encoded = serde_json::to_string(&event).unwrap();
        assert_eq!(encoded.matches("\"type\"").count(), 2);
        assert_eq!(
            serde_json::from_str::<DaemonEvent>(&encoded).unwrap(),
            event
        );
    }

    #[test]
    fn compatibility_validation_rejects_duplicate_oversized_and_unsupported_evidence() {
        let mut duplicate = compatibility_snapshot_fixture(1);
        duplicate
            .capabilities
            .push(duplicate.capabilities[0].clone());
        assert!(matches!(
            duplicate.validate(),
            Err(CompatibilityProtocolError::DuplicateCapability(_))
        ));

        let mut oversized = compatibility_snapshot_fixture(1);
        oversized.capabilities[0].evidence[0].argv =
            vec!["argument".into(); MAX_COMPATIBILITY_ARGV + 1];
        assert_eq!(
            oversized.validate(),
            Err(CompatibilityProtocolError::Oversized("evidence argv"))
        );

        let mut contradicted = compatibility_snapshot_fixture(1);
        contradicted.capabilities[0].state = CompatibilityStateData::Unavailable {
            reason: CompatibilityReasonData {
                code: "command_missing".into(),
                message: "The command is unavailable.".into(),
                requirement: Some("bitbake-getvar --value".into()),
            },
        };
        contradicted.capabilities[0].implementation = None;
        assert!(matches!(
            contradicted.validate(),
            Err(CompatibilityProtocolError::EvidenceMismatch(_))
        ));
    }

    #[test]
    fn compatibility_unknown_wire_values_fail_closed() {
        let state: CompatibilityStateData =
            serde_json::from_str(r#"{"state":"available_in_a_future_protocol"}"#).unwrap();
        assert_eq!(state, CompatibilityStateData::UnknownWireState);
        assert!(!state.is_enabled());

        let evidence_kind: CompatibilityEvidenceKind =
            serde_json::from_str(r#""future_probe""#).unwrap();
        let evidence_outcome: CompatibilityEvidenceOutcome =
            serde_json::from_str(r#""future_outcome""#).unwrap();
        assert_eq!(evidence_kind, CompatibilityEvidenceKind::Unknown);
        assert_eq!(evidence_outcome, CompatibilityEvidenceOutcome::Unknown);
    }

    #[test]
    fn compatibility_events_replace_newer_snapshots_and_reject_stale_generations() {
        let mut initial = daemon_snapshot_fixture();
        initial.compatibility = Some(compatibility_snapshot_fixture(1));
        let mut journal = DaemonSnapshotJournal::new(initial, DaemonSnapshotLimits::default())
            .expect("valid compatibility snapshot");

        journal
            .publish(DaemonEvent::CompatibilityChanged(Box::new(
                compatibility_snapshot_fixture(2),
            )))
            .unwrap();
        assert_eq!(
            journal
                .snapshot()
                .compatibility
                .as_ref()
                .unwrap()
                .generation,
            2
        );

        assert!(matches!(
            journal.publish(DaemonEvent::CompatibilityChanged(Box::new(
                compatibility_snapshot_fixture(2)
            ))),
            Err(DaemonSnapshotError::StaleCompatibilityGeneration {
                current: 2,
                received: 2
            })
        ));
        assert_eq!(journal.snapshot().sequence, 1);
    }

    #[test]
    fn daemon_protocol_negotiates_versions_and_capabilities_explicitly() {
        assert_eq!(
            negotiate_version(
                ProtocolVersion { major: 1, minor: 0 },
                ProtocolVersion { major: 1, minor: 4 },
                ProtocolVersion { major: 1, minor: 2 },
            )
            .unwrap(),
            ProtocolVersion { major: 1, minor: 2 }
        );
        assert!(matches!(
            negotiate_version(
                ProtocolVersion { major: 2, minor: 0 },
                ProtocolVersion { major: 2, minor: 0 },
                ProtocolVersion::CURRENT,
            ),
            Err(DaemonProtocolError::IncompatibleVersion)
        ));
        assert_eq!(
            negotiate_capabilities(
                &[Capability::PtySessions, Capability::StateSnapshots],
                &[Capability::StateSnapshots, Capability::BackgroundJobs],
            )
            .unwrap(),
            vec![Capability::StateSnapshots]
        );

        let future: Capability = serde_json::from_str("\"future_capability\"").unwrap();
        assert_eq!(future, Capability::Unknown);
        let future_event: DaemonEvent =
            serde_json::from_str(r#"{"type":"future_optional_event"}"#).unwrap();
        assert_eq!(future_event, DaemonEvent::Unknown);
    }

    #[test]
    fn daemon_protocol_round_trips_snapshot_event_and_correlated_command() {
        let snapshot = DaemonSnapshot {
            daemon_instance_id: DaemonInstanceId([7; 16]),
            sequence: 42,
            generation: 9,
            workspace: None,
            project_profile: ProjectProfileSummary::Absent,
            bitbake: BitBakeState {
                lifecycle: LifecycleState::Running,
                version: Some("2.8.1".into()),
                capabilities: vec![BitBakeCapability::WorkspaceInspection],
                diagnostic: None,
            },
            compatibility: None,
            jobs: Vec::new(),
            raw_executions: Vec::new(),
            raw_history: Vec::new(),
            pty_sessions: Vec::new(),
            pty_screens: Vec::new(),
            clients: vec![ClientSummary {
                id: client_id(1),
                name: "ssh-client".into(),
                attached_unix_ms: 1,
                last_seen_unix_ms: 2,
            }],
            recent_logs: Vec::new(),
            build_events: Vec::new(),
            build_progress: None,
            recovery_warnings: Vec::new(),
        };
        let message = ServerMessage::Attached {
            snapshot,
            replayed_through: 42,
        };
        assert_eq!(
            decode_frame::<ServerMessage>(&encode_frame(&message).unwrap()).unwrap(),
            message
        );

        let command = ClientMessage::Command(CommandRequest {
            request_id: RequestId(81),
            expected_generation: Some(9),
            command: DaemonCommand::TakePtyControl {
                session_id: PtySessionId(3),
                expected_epoch: 0,
            },
        });
        assert_eq!(
            decode_frame::<ClientMessage>(&encode_frame(&command).unwrap()).unwrap(),
            command
        );
    }

    #[test]
    fn daemon_protocol_frames_partial_messages_and_rejects_oversize() {
        let first = encode_frame(&ClientMessage::Detach).unwrap();
        let second = encode_frame(&ClientMessage::Pong { nonce: 4 }).unwrap();
        let mut decoder = FrameDecoder::default();
        assert!(decoder.push(&first[..3]).unwrap().is_empty());
        let mut remainder = first[3..].to_vec();
        remainder.extend_from_slice(&second);
        let frames = decoder.push(&remainder).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(
            decode_frame::<ClientMessage>(&frames[0]).unwrap(),
            ClientMessage::Detach
        );
        assert_eq!(
            decode_frame::<ClientMessage>(&frames[1]).unwrap(),
            ClientMessage::Pong { nonce: 4 }
        );
        assert_eq!(decoder.pending_len(), 0);

        let mut oversized = FrameDecoder::default();
        assert!(matches!(
            oversized.push(&((MAX_FRAME_BYTES as u32 + 1).to_be_bytes())),
            Err(DaemonProtocolError::TooLarge)
        ));
    }

    #[test]
    fn daemon_protocol_covers_reconnect_stale_writer_layout_and_mouse() {
        let attach = ClientMessage::Attach {
            workspace: None,
            subscription: Subscription {
                state: true,
                jobs: true,
                logs: false,
                pty_sessions: vec![PtySessionId(8)],
            },
            resume: Some(ResumeCursor {
                daemon_instance_id: DaemonInstanceId([9; 16]),
                last_sequence: 77,
            }),
        };
        assert_eq!(
            decode_frame::<ClientMessage>(&encode_frame(&attach).unwrap()).unwrap(),
            attach
        );

        let stale = ServerMessage::CommandResult(CommandResult {
            request_id: RequestId(5),
            outcome: CommandOutcome::Rejected {
                code: ProtocolErrorCode::StaleGeneration,
                message: "snapshot replaced".into(),
                current_generation: 12,
            },
        });
        assert_eq!(
            decode_frame::<ServerMessage>(&encode_frame(&stale).unwrap()).unwrap(),
            stale
        );

        for message in [
            ClientMessage::Layout {
                event: ClientLayoutEvent::AttachSession {
                    pane_id: PaneId(2),
                    session_id: PtySessionId(8),
                },
            },
            ClientMessage::Mouse {
                event: ServerMouseEvent {
                    session_id: PtySessionId(8),
                    writer_epoch: 3,
                    kind: MouseEventKind::Drag,
                    button: 1,
                    column: 40,
                    row: 12,
                    modifiers: 0,
                },
            },
        ] {
            assert_eq!(
                decode_frame::<ClientMessage>(&encode_frame(&message).unwrap()).unwrap(),
                message
            );
        }
    }

    #[test]
    fn multi_client_state_keeps_identity_global_and_layout_local() {
        let mut snapshot = daemon_snapshot_fixture();
        snapshot.clients = vec![
            ClientSummary {
                id: ClientId([1; 16]),
                name: "left".into(),
                attached_unix_ms: 1,
                last_seen_unix_ms: 2,
            },
            ClientSummary {
                id: ClientId([2; 16]),
                name: "right".into(),
                attached_unix_ms: 1,
                last_seen_unix_ms: 2,
            },
        ];
        let event = DaemonEvent::ClientChanged(snapshot.clients[1].clone());
        let sequenced = SequencedEvent {
            sequence: 1,
            generation: 1,
            event,
        };
        apply_sequenced_event(&mut snapshot, &sequenced).unwrap();
        assert_eq!(snapshot.clients.len(), 2);
        assert_ne!(snapshot.clients[0].id, snapshot.clients[1].id);
    }

    #[test]
    fn multi_client_fanout_replays_global_events_from_independent_cursors() {
        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        journal
            .publish(DaemonEvent::RecoveryWarning {
                message: "shared".into(),
            })
            .unwrap();
        let cursor = ResumeCursor {
            daemon_instance_id: journal.snapshot().daemon_instance_id,
            last_sequence: 0,
        };
        let first = journal.synchronize(Some(cursor));
        let second = journal.synchronize(Some(cursor));
        let events = |sync| match sync {
            DaemonSnapshotSync::Replay { events, .. } => events,
            _ => panic!("expected replay"),
        };
        assert_eq!(events(first), events(second));
    }

    #[test]
    fn next_generation_pty_screen_is_bounded_and_retained_for_reattach() {
        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        let screen = PtyScreenSnapshot {
            session_id: PtySessionId(9),
            dimensions: TerminalDimensions {
                columns: 20,
                rows: 4,
            },
            cursor_column: 3,
            cursor_row: 1,
            cursor_hidden: false,
            scrollback_offset: 0,
            cells: Vec::new(),
            scrollback_lines: 7,
            dropped_line_feeds_lower_bound: 0,
        };
        journal
            .publish(DaemonEvent::PtyScreen(screen.clone()))
            .unwrap();
        assert_eq!(journal.snapshot().pty_screens, vec![screen.clone()]);
        let mut replacement = screen;
        replacement.cells = vec![PtyScreenCell {
            index: 0,
            contents: "updated".into(),
            foreground: PtyTerminalColor::Default,
            background: PtyTerminalColor::Default,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            inverse: false,
            wide: false,
            wide_continuation: false,
        }];
        journal
            .publish(DaemonEvent::PtyScreen(replacement.clone()))
            .unwrap();
        assert_eq!(journal.snapshot().pty_screens, vec![replacement]);

        let invalid = PtyScreenSnapshot {
            session_id: PtySessionId(10),
            dimensions: TerminalDimensions {
                columns: 2,
                rows: 1,
            },
            cursor_column: 0,
            cursor_row: 0,
            cursor_hidden: false,
            scrollback_offset: 0,
            cells: vec![PtyScreenCell {
                index: 2,
                contents: "outside".into(),
                foreground: PtyTerminalColor::Default,
                background: PtyTerminalColor::Default,
                bold: false,
                dim: false,
                italic: false,
                underline: false,
                inverse: false,
                wide: false,
                wide_continuation: false,
            }],
            scrollback_lines: 0,
            dropped_line_feeds_lower_bound: 0,
        };
        assert!(matches!(
            journal.publish(DaemonEvent::PtyScreen(invalid)),
            Err(DaemonSnapshotError::InvalidPtyScreen(PtySessionId(10)))
        ));
    }

    #[test]
    fn daemon_snapshot_is_gap_free_bounded_and_replays_only_retained_events() {
        let mut journal = DaemonSnapshotJournal::new(
            daemon_snapshot_fixture(),
            DaemonSnapshotLimits {
                retained_events: 2,
                recent_logs: 2,
                snapshot_bytes: MAX_FRAME_BYTES,
            },
        )
        .unwrap();
        for index in 1..=3 {
            let event = journal
                .publish(DaemonEvent::Log(LogRecord {
                    source: "test".into(),
                    severity: LogSeverity::Info,
                    message: format!("event-{index}"),
                    unix_ms: index,
                    recipe: None,
                    task: None,
                    path: None,
                    build: None,
                }))
                .unwrap();
            assert_eq!(event.sequence, index);
            assert_eq!(event.generation, index);
        }
        assert_eq!(journal.snapshot().sequence, 3);
        assert_eq!(journal.snapshot().recent_logs.len(), 2);
        assert_eq!(journal.snapshot().recent_logs[0].message, "event-2");

        let replay = journal.synchronize(Some(ResumeCursor {
            daemon_instance_id: DaemonInstanceId([7; 16]),
            last_sequence: 1,
        }));
        assert!(matches!(
            replay,
            DaemonSnapshotSync::Replay {
                ref events,
                replayed_through: 3
            } if events.iter().map(|event| event.sequence).collect::<Vec<_>>() == vec![2, 3]
        ));
        assert!(matches!(
            journal.synchronize_bounded(
                ResumeCursor {
                    daemon_instance_id: DaemonInstanceId([7; 16]),
                    last_sequence: 1,
                },
                1,
            ),
            DaemonSnapshotSync::Replay {
                ref events,
                replayed_through: 2
            } if events.len() == 1 && events[0].sequence == 2
        ));
        assert!(matches!(
            journal.synchronize(Some(ResumeCursor {
                daemon_instance_id: DaemonInstanceId([7; 16]),
                last_sequence: 0,
            })),
            DaemonSnapshotSync::Replace {
                reason: SnapshotReplacementReason::HistoryExpired,
                ..
            }
        ));
        assert!(matches!(
            journal.synchronize(Some(ResumeCursor {
                daemon_instance_id: DaemonInstanceId([8; 16]),
                last_sequence: 3,
            })),
            DaemonSnapshotSync::Replace {
                reason: SnapshotReplacementReason::DaemonInstanceChanged,
                ..
            }
        ));

        let mut client = daemon_snapshot_fixture();
        let gap = SequencedEvent {
            sequence: 2,
            generation: 2,
            event: DaemonEvent::Unknown,
        };
        assert!(matches!(
            apply_sequenced_event(&mut client, &gap),
            Err(DaemonSnapshotError::EventGap { .. })
        ));
        assert_eq!(client.sequence, 0);
    }

    #[test]
    fn daemon_snapshot_evicts_oldest_logs_to_preserve_byte_bound() {
        let mut journal = DaemonSnapshotJournal::new(
            daemon_snapshot_fixture(),
            DaemonSnapshotLimits {
                retained_events: 16,
                recent_logs: 16,
                snapshot_bytes: 2_048,
            },
        )
        .unwrap();
        for index in 1..=4 {
            journal
                .publish(DaemonEvent::Log(LogRecord {
                    source: "bitbake".into(),
                    severity: LogSeverity::Info,
                    message: "x".repeat(900),
                    unix_ms: index,
                    recipe: None,
                    task: None,
                    path: None,
                    build: None,
                }))
                .unwrap();
        }
        let encoded = serde_json::to_vec(journal.snapshot()).unwrap();
        assert!(encoded.len() <= 2_048);
        assert!(journal.snapshot().recent_logs.len() < 4);
        assert_eq!(journal.snapshot().sequence, 4);
    }

    #[test]
    fn daemon_snapshot_rejects_invalid_limits_and_oversized_snapshots() {
        assert!(matches!(
            DaemonSnapshotJournal::new(
                daemon_snapshot_fixture(),
                DaemonSnapshotLimits {
                    retained_events: 0,
                    ..DaemonSnapshotLimits::default()
                },
            ),
            Err(DaemonSnapshotError::InvalidLimit("retained events"))
        ));
        assert!(matches!(
            DaemonSnapshotJournal::new(
                daemon_snapshot_fixture(),
                DaemonSnapshotLimits {
                    snapshot_bytes: 1,
                    ..DaemonSnapshotLimits::default()
                },
            ),
            Err(DaemonSnapshotError::SnapshotTooLarge { .. })
        ));

        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        assert!(matches!(
            journal.publish(DaemonEvent::Log(LogRecord {
                source: "test".into(),
                severity: LogSeverity::Error,
                message: "x".repeat(MAX_FRAME_BYTES),
                unix_ms: 0,
                recipe: None,
                task: None,
                path: None,
                build: None,
            })),
            Err(DaemonSnapshotError::EventTooLarge { .. })
        ));
        assert_eq!(journal.snapshot().sequence, 0);
    }

    #[test]
    fn snapshot_progress_keeps_counts_after_eviction_and_duplicate_completion() {
        let mut snapshot = daemon_snapshot_fixture();
        apply_build_event(
            &mut snapshot,
            DaemonBuildEvent::Reset {
                targets: vec!["image".into()],
            },
        );
        apply_build_event(
            &mut snapshot,
            DaemonBuildEvent::TaskQueued {
                recipe: "seed".into(),
                task: "do_compile".into(),
                worker: None,
                stats: Some(TaskStatsData {
                    completed: 2_339,
                    total: 6_812,
                    active: 1,
                    failed: 0,
                }),
            },
        );
        let count = MAX_DAEMON_BUILD_EVENTS + 8;
        for index in 0..count {
            let event = DaemonBuildEvent::TaskCompleted {
                recipe: format!("recipe-{index}"),
                task: "do_compile".into(),
                success: index % 2 == 0,
                started_unix_ms: None,
                finished_unix_ms: None,
            };
            apply_build_event(&mut snapshot, event.clone());
            apply_build_event(&mut snapshot, event);
        }
        assert_eq!(snapshot.build_events.len(), MAX_DAEMON_BUILD_EVENTS);
        assert_eq!(
            snapshot.build_progress,
            Some(DaemonBuildProgress {
                completed: 2_339 + count,
                total: Some(6_812),
                ..Default::default()
            })
        );
        assert!(!snapshot.build_events.iter().any(|event| matches!(event,
            DaemonBuildEvent::TaskCompleted { recipe, .. } if recipe == "recipe-0")));
        let encoded = encode_frame(&ServerMessage::Snapshot(snapshot.clone())).unwrap();
        assert!(encoded.len() < MAX_FRAME_BYTES);
        assert_eq!(
            decode_frame::<ServerMessage>(&encoded).unwrap(),
            ServerMessage::Snapshot(snapshot.clone())
        );
        apply_build_event(
            &mut snapshot,
            DaemonBuildEvent::Completed {
                success: false,
                exit_code: Some(1),
                finished_unix_ms: None,
            },
        );
        assert_eq!(snapshot.build_progress.unwrap().completed, 2_339 + count);
        apply_build_event(
            &mut snapshot,
            DaemonBuildEvent::Completed {
                success: true,
                exit_code: Some(0),
                finished_unix_ms: None,
            },
        );
        assert_eq!(snapshot.build_progress.unwrap().completed, 6_812);
        apply_build_event(
            &mut snapshot,
            DaemonBuildEvent::Reset {
                targets: vec!["next".into()],
            },
        );
        assert_eq!(
            snapshot.build_progress,
            Some(DaemonBuildProgress::default())
        );
    }

    #[test]
    fn snapshot_progress_preserves_unknown_totals_and_legacy_absence() {
        let mut snapshot = daemon_snapshot_fixture();
        let encoded = serde_json::to_value(&snapshot).unwrap();
        assert!(encoded.get("build_progress").is_none());
        let legacy: DaemonSnapshot = serde_json::from_value(encoded).unwrap();
        assert_eq!(legacy.build_progress, None);
        apply_build_event(
            &mut snapshot,
            DaemonBuildEvent::TaskCompleted {
                recipe: "legacy".into(),
                task: "do_compile".into(),
                success: true,
                started_unix_ms: None,
                finished_unix_ms: None,
            },
        );
        assert_eq!(snapshot.build_progress, None);
        apply_build_event(
            &mut snapshot,
            DaemonBuildEvent::Started {
                started_unix_ms: None,
            },
        );
        apply_build_event(
            &mut snapshot,
            DaemonBuildEvent::TaskQueued {
                recipe: "current".into(),
                task: "do_compile".into(),
                worker: None,
                stats: Some(TaskStatsData {
                    completed: 42,
                    total: 0,
                    active: 1,
                    failed: 0,
                }),
            },
        );
        apply_build_event(
            &mut snapshot,
            DaemonBuildEvent::Completed {
                success: true,
                exit_code: Some(0),
                finished_unix_ms: None,
            },
        );
        assert_eq!(
            snapshot.build_progress,
            Some(DaemonBuildProgress {
                completed: 42,
                total: None,
                ..Default::default()
            })
        );
        for invalid in [
            r#"{"completed":-1,"total":6812}"#,
            r#"{"completed":1,"total":-1}"#,
            r#"{"completed":1,"total":"unknown"}"#,
            r#"{"completed":1.5,"total":6812}"#,
        ] {
            assert!(
                serde_json::from_str::<DaemonBuildProgress>(invalid).is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn snapshot_timing_compacts_start_into_completion_and_freezes_duplicates() {
        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        let publish = |journal: &mut DaemonSnapshotJournal, event| {
            journal.publish(DaemonEvent::Build(event)).unwrap()
        };
        publish(
            &mut journal,
            DaemonBuildEvent::Started {
                started_unix_ms: Some(1000),
            },
        );
        publish(
            &mut journal,
            DaemonBuildEvent::Started {
                started_unix_ms: Some(9000),
            },
        );
        let started = DaemonBuildEvent::TaskStarted {
            recipe: "recipe".into(),
            task: "do_compile".into(),
            started_unix_ms: Some(2000),
            pid: None,
            worker: None,
            log_path: None,
            stats: None,
        };
        publish(&mut journal, started);
        let completed = |finished| DaemonBuildEvent::TaskCompleted {
            recipe: "recipe".into(),
            task: "do_compile".into(),
            success: true,
            started_unix_ms: None,
            finished_unix_ms: Some(finished),
        };
        let first = publish(&mut journal, completed(2500));
        let repeated = publish(&mut journal, completed(9900));
        assert_eq!(first.event, repeated.event);
        assert!(matches!(
            first.event,
            DaemonEvent::Build(DaemonBuildEvent::TaskCompleted {
                started_unix_ms: Some(2000),
                finished_unix_ms: Some(2500),
                ..
            })
        ));
        assert!(
            !journal
                .snapshot()
                .build_events
                .iter()
                .any(|event| matches!(event, DaemonBuildEvent::TaskStarted { .. }))
        );
        let terminal = |finished| DaemonBuildEvent::Completed {
            success: true,
            exit_code: Some(0),
            finished_unix_ms: Some(finished),
        };
        let first = publish(&mut journal, terminal(5000));
        let repeated = publish(&mut journal, terminal(10000));
        assert_eq!(first.event, repeated.event);
        assert!(
            journal
                .snapshot()
                .build_events
                .iter()
                .filter(|event| matches!(event, DaemonBuildEvent::Started { .. }))
                .all(|event| matches!(
                    event,
                    DaemonBuildEvent::Started {
                        started_unix_ms: Some(1000)
                    }
                ))
        );
        publish(
            &mut journal,
            DaemonBuildEvent::Reset {
                targets: vec!["next".into()],
            },
        );
        let next = publish(
            &mut journal,
            DaemonBuildEvent::Started {
                started_unix_ms: Some(20000),
            },
        );
        assert!(matches!(
            next.event,
            DaemonEvent::Build(DaemonBuildEvent::Started {
                started_unix_ms: Some(20000)
            })
        ));
    }

    #[test]
    fn task_identity_unresolved_job_statistics_preserve_aggregate_without_rows() {
        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        let job = |kind, completed| {
            DaemonEvent::JobChanged(JobSummary {
                id: JobId(99),
                kind,
                label: "build".into(),
                lifecycle: LifecycleState::Running,
                progress_current: Some(completed),
                progress_total: Some(6812),
                exit_code: None,
            })
        };
        journal.publish(job(JobKind::BitBakeBuild, 100)).unwrap();
        assert_eq!(
            journal.snapshot().build_progress,
            None,
            "legacy absence is not a current build"
        );
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::Started {
                started_unix_ms: None,
            }))
            .unwrap();
        journal.publish(job(JobKind::BitBakeBuild, 2340)).unwrap();
        assert_eq!(
            journal.snapshot().build_progress,
            Some(DaemonBuildProgress {
                completed: 2340,
                total: Some(6812),
                ..Default::default()
            })
        );
        assert_eq!(journal.snapshot().build_events.len(), 1);
        journal.publish(job(JobKind::Qa, 4000)).unwrap();
        assert_eq!(journal.snapshot().build_progress.unwrap().completed, 2340);
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::Disconnected))
            .unwrap();
        journal.publish(job(JobKind::BitBakeBuild, 4500)).unwrap();
        assert_eq!(
            journal.snapshot().build_progress.unwrap().completed,
            2340,
            "lost authority cannot take later progress"
        );
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::Completed {
                success: false,
                exit_code: Some(1),
                finished_unix_ms: None,
            }))
            .unwrap();
        journal.publish(job(JobKind::BitBakeBuild, 5000)).unwrap();
        assert_eq!(
            journal.snapshot().build_progress.unwrap().completed,
            2340,
            "late progress cannot change terminal authority"
        );
    }

    #[test]
    fn snapshot_timing_retained_rows_stay_bounded_after_eviction() {
        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::Started {
                started_unix_ms: Some(0),
            }))
            .unwrap();
        let count = MAX_DAEMON_BUILD_EVENTS + 4;
        for index in 0..count {
            journal
                .publish(DaemonEvent::Build(DaemonBuildEvent::TaskStarted {
                    recipe: format!("recipe-{index}"),
                    task: "do_compile".into(),
                    started_unix_ms: Some(index as u64),
                    pid: None,
                    worker: None,
                    log_path: None,
                    stats: None,
                }))
                .unwrap();
            journal
                .publish(DaemonEvent::Build(DaemonBuildEvent::TaskCompleted {
                    recipe: format!("recipe-{index}"),
                    task: "do_compile".into(),
                    success: true,
                    started_unix_ms: None,
                    finished_unix_ms: Some(index as u64 + 100),
                }))
                .unwrap();
        }
        assert_eq!(
            journal.snapshot().build_events.len(),
            MAX_DAEMON_BUILD_EVENTS
        );
        assert!(
            matches!(journal.snapshot().build_events.last(), Some(DaemonBuildEvent::TaskCompleted {
            started_unix_ms: Some(start), finished_unix_ms: Some(end), ..
        }) if *start == (count - 1) as u64 && *end == (count - 1) as u64 + 100)
        );
        let message = ServerMessage::Snapshot(journal.snapshot().clone());
        let frame = encode_frame(&message).unwrap();
        assert!(frame.len() < MAX_FRAME_BYTES);
        assert_eq!(decode_frame::<ServerMessage>(&frame).unwrap(), message);
    }

    #[test]
    fn snapshot_timing_keeps_legacy_wire_shapes_and_rejects_malformed_times() {
        let legacy = r#"{"type":"started"}"#;
        let decoded: DaemonBuildEvent = serde_json::from_str(legacy).unwrap();
        assert_eq!(
            decoded,
            DaemonBuildEvent::Started {
                started_unix_ms: None
            }
        );
        assert_eq!(serde_json::to_string(&decoded).unwrap(), legacy);
        #[derive(Debug, PartialEq, serde::Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case")]
        enum LegacyEvent {
            Started,
            Completed {
                success: bool,
                exit_code: Option<i32>,
            },
        }
        let started = DaemonBuildEvent::Started {
            started_unix_ms: Some(123),
        };
        assert_eq!(
            serde_json::from_value::<LegacyEvent>(serde_json::to_value(started).unwrap()).unwrap(),
            LegacyEvent::Started
        );
        let completed = DaemonBuildEvent::Completed {
            success: true,
            exit_code: Some(0),
            finished_unix_ms: Some(456),
        };
        assert_eq!(
            serde_json::from_value::<LegacyEvent>(serde_json::to_value(completed).unwrap())
                .unwrap(),
            LegacyEvent::Completed {
                success: true,
                exit_code: Some(0)
            }
        );
        for malformed in [
            r#"{"type":"started","started_unix_ms":-1}"#,
            r#"{"type":"started","started_unix_ms":1.5}"#,
            r#"{"type":"started","started_unix_ms":"now"}"#,
            r#"{"type":"started","started_unix_ms":18446744073709551616}"#,
        ] {
            assert!(
                serde_json::from_str::<DaemonBuildEvent>(malformed).is_err(),
                "{malformed}"
            );
        }
    }

    #[test]
    fn daemon_build_snapshot_retains_typed_attach_progress() {
        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::Reset {
                targets: vec!["core-image-minimal".into()],
            }))
            .unwrap();
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::TaskStarted {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                pid: Some(42),
                worker: Some("worker-1".into()),
                log_path: Some("/build/temp/log.do_compile".into()),
                stats: Some(TaskStatsData {
                    completed: 102,
                    total: 4090,
                    active: 8,
                    failed: 0,
                }),
                started_unix_ms: None,
            }))
            .unwrap();
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::TaskProgress {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(77),
            }))
            .unwrap();
        assert_eq!(journal.snapshot().build_events.len(), 3);
        assert!(matches!(
            journal.snapshot().build_events.last(),
            Some(DaemonBuildEvent::TaskProgress {
                progress: Some(77),
                ..
            })
        ));
        let encoded = encode_frame(&ServerMessage::Snapshot(journal.snapshot().clone())).unwrap();
        assert!(encoded.len() < MAX_FRAME_BYTES);
    }

    #[test]
    fn daemon_journal_uses_conservative_headroom_between_snapshot_serializations() {
        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::TaskStarted {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                pid: Some(42),
                worker: None,
                log_path: None,
                stats: None,
                started_unix_ms: None,
            }))
            .unwrap();
        for progress in (0..100).cycle().take(10_000) {
            journal
                .publish(DaemonEvent::Build(DaemonBuildEvent::TaskProgress {
                    recipe: "busybox".into(),
                    task: "do_compile".into(),
                    progress: Some(progress),
                }))
                .unwrap();
        }
        let metrics = journal.ipc_metrics();
        let exact = serde_json::to_vec(journal.snapshot()).unwrap().len();
        assert_eq!(metrics.published_events, 10_001);
        assert!(metrics.published_event_bytes > 0);
        assert!(metrics.snapshot_serializations <= 2, "{metrics:?}");
        assert!(exact <= metrics.snapshot_bytes_upper_bound);
        assert!(metrics.snapshot_bytes_upper_bound <= MAX_FRAME_BYTES);
        assert!(matches!(
            journal.snapshot().build_events.last(),
            Some(DaemonBuildEvent::TaskProgress {
                progress: Some(99),
                ..
            })
        ));
    }

    #[test]
    fn daemon_journal_updates_high_rate_job_progress_without_full_snapshot_serialization() {
        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        for completed in 0..10_000 {
            journal
                .publish(DaemonEvent::JobChanged(JobSummary {
                    id: JobId(41),
                    kind: JobKind::BitBakeBuild,
                    label: "BitBake build core-image-sato".into(),
                    lifecycle: LifecycleState::Running,
                    progress_current: Some(completed),
                    progress_total: Some(10_000),
                    exit_code: None,
                }))
                .unwrap();
        }
        let metrics = journal.ipc_metrics();
        let exact = serde_json::to_vec(journal.snapshot()).unwrap().len();
        assert_eq!(metrics.published_events, 10_000);
        assert!(metrics.snapshot_serializations <= 2, "{metrics:?}");
        assert!(exact <= metrics.snapshot_bytes_upper_bound);
        assert_eq!(journal.snapshot().jobs[0].progress_current, Some(9_999));
    }

    fn journal_telemetry(uptime_seconds: u64) -> DaemonEvent {
        DaemonEvent::Telemetry(DaemonTelemetry {
            uptime_seconds,
            bitbake: LifecycleState::Running,
            connected_clients: 2,
            active_jobs: 1,
            pty_sessions: 0,
            queue_depth: 1,
            pressure: DaemonPressureCounters::default(),
            memory_bytes: Some(24 * 1024 * 1024),
            recovery: DaemonRecoveryState::CleanStart,
        })
    }

    #[test]
    fn daemon_journal_telemetry_replays_without_serializing_unchanged_snapshot() {
        let mut snapshot = daemon_snapshot_fixture();
        snapshot.recovery_warnings = vec!["retained metadata with \\".repeat(8_192)];
        let mut replica = snapshot.clone();
        let cursor = ResumeCursor {
            daemon_instance_id: snapshot.daemon_instance_id,
            last_sequence: snapshot.sequence,
        };
        let mut journal =
            DaemonSnapshotJournal::new(snapshot.clone(), DaemonSnapshotLimits::default()).unwrap();
        let initial_serializations = journal.ipc_metrics().snapshot_serializations;
        for uptime in 1..=100 {
            let published = journal.publish(journal_telemetry(uptime)).unwrap();
            assert_eq!(published.sequence, uptime);
            assert_eq!(published.generation, uptime);
            assert_eq!(published.event, journal_telemetry(uptime));
        }
        assert_eq!(
            journal.ipc_metrics().snapshot_serializations,
            initial_serializations,
            "telemetry must not serialize unchanged retained metadata while headroom exists"
        );
        let DaemonSnapshotSync::Replay {
            events,
            replayed_through,
        } = journal.synchronize(Some(cursor))
        else {
            panic!("retained telemetry must replay incrementally");
        };
        assert_eq!(events.len(), 100);
        assert_eq!(replayed_through, 100);
        for event in events {
            let frame = encode_frame(&ServerMessage::Event(event.clone())).unwrap();
            assert_eq!(
                decode_frame::<ServerMessage>(&frame).unwrap(),
                ServerMessage::Event(event.clone())
            );
            apply_sequenced_event(&mut replica, &event).unwrap();
        }
        snapshot.sequence = 100;
        snapshot.generation = 100;
        assert_eq!(journal.snapshot(), &snapshot);
        assert_eq!(replica, snapshot);
        assert_eq!(journal.ipc_metrics().published_events, 100);
        let exact = serde_json::to_vec(journal.snapshot()).unwrap().len();
        assert!(exact <= journal.ipc_metrics().snapshot_bytes_upper_bound);
        assert!(journal.ipc_metrics().snapshot_bytes_upper_bound <= MAX_FRAME_BYTES);
    }

    #[test]
    fn daemon_journal_telemetry_remeasures_at_the_same_hard_byte_limit() {
        let snapshot = daemon_snapshot_fixture();
        let limit = serde_json::to_vec(&snapshot).unwrap().len() + 4_096;
        let mut journal = DaemonSnapshotJournal::new(
            snapshot,
            DaemonSnapshotLimits {
                snapshot_bytes: limit,
                retained_events: 4,
                ..DaemonSnapshotLimits::default()
            },
        )
        .unwrap();
        for uptime in 1..=100 {
            journal.publish(journal_telemetry(uptime)).unwrap();
            let exact = serde_json::to_vec(journal.snapshot()).unwrap().len();
            assert!(exact <= journal.ipc_metrics().snapshot_bytes_upper_bound);
            assert!(journal.ipc_metrics().snapshot_bytes_upper_bound <= limit);
            assert!(journal.events.len() <= 4);
        }
        let serializations = journal.ipc_metrics().snapshot_serializations;
        assert!(
            serializations > 1,
            "size ledger must eventually be remeasured"
        );
        assert!(serializations < 100, "headroom must amortize snapshot work");
    }

    #[test]
    fn daemon_journal_telemetry_size_rejection_preserves_snapshot_and_replay() {
        let mut snapshot = daemon_snapshot_fixture();
        snapshot.sequence = 8;
        snapshot.generation = 8;
        let limit = serde_json::to_vec(&snapshot).unwrap().len() + 1;
        let mut journal = DaemonSnapshotJournal::new(
            snapshot,
            DaemonSnapshotLimits {
                snapshot_bytes: limit,
                ..DaemonSnapshotLimits::default()
            },
        )
        .unwrap();
        journal.publish(journal_telemetry(1)).unwrap();
        let before = journal.snapshot().clone();
        let before_events = journal.events.clone();
        let before_metrics = journal.ipc_metrics();
        // Both sequence counters grow from one digit to two; the hard limit
        // leaves only one byte, so even non-retained telemetry must fail.
        assert!(matches!(
            journal.publish(journal_telemetry(2)),
            Err(DaemonSnapshotError::SnapshotTooLarge { .. })
        ));
        assert_eq!(journal.snapshot(), &before);
        assert_eq!(journal.events, before_events);
        assert_eq!(
            journal.ipc_metrics().published_events,
            before_metrics.published_events
        );
        assert_eq!(
            journal.ipc_metrics().published_event_bytes,
            before_metrics.published_event_bytes
        );
        assert_eq!(
            journal.ipc_metrics().snapshot_bytes_upper_bound,
            before_metrics.snapshot_bytes_upper_bound
        );
    }

    #[test]
    fn daemon_journal_telemetry_counter_exhaustion_is_transactional() {
        for sequence_exhausted in [true, false] {
            let mut snapshot = daemon_snapshot_fixture();
            if sequence_exhausted {
                snapshot.sequence = u64::MAX;
            } else {
                snapshot.generation = u64::MAX;
            }
            let mut journal =
                DaemonSnapshotJournal::new(snapshot.clone(), DaemonSnapshotLimits::default())
                    .unwrap();
            let result = journal.publish(journal_telemetry(1));
            if sequence_exhausted {
                assert!(matches!(
                    result,
                    Err(DaemonSnapshotError::SequenceExhausted)
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(DaemonSnapshotError::GenerationExhausted)
                ));
            }
            assert_eq!(journal.snapshot(), &snapshot);
            assert!(journal.events.is_empty());
            assert_eq!(journal.ipc_metrics().published_events, 0);
            assert_eq!(journal.ipc_metrics().snapshot_serializations, 1);
        }
    }

    #[test]
    fn daemon_workspace_event_updates_persistent_workspace_identity() {
        let mut journal =
            DaemonSnapshotJournal::new(daemon_snapshot_fixture(), DaemonSnapshotLimits::default())
                .unwrap();
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::Workspace {
                data: WorkspaceData {
                    build_dir: Some("/srv/yocto/build".into()),
                    source_dir: Some("/srv/yocto/poky".into()),
                    variables: std::collections::HashMap::new(),
                    variable_provenance: std::collections::HashMap::new(),
                    variable_provenance_chain: std::collections::HashMap::new(),
                    bitbake_version: Some("2.18.0".into()),
                    release: Some("6.0.2".into()),
                    layers: Vec::new(),
                    recipes: Vec::new(),
                },
            }))
            .unwrap();

        let identity = journal.snapshot().workspace.as_ref().unwrap();
        assert_eq!(identity.canonical_source, "/srv/yocto/poky");
        assert_eq!(identity.canonical_build, "/srv/yocto/build");
        assert_eq!(identity.identity_hash.len(), 16);
    }

    fn raw_execution_request_fixture() -> RawExecutionRequestData {
        RawExecutionRequestData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            request_id: "raw-request:protocol-1".into(),
            catalog_version: 1,
            command_id: "build.target".into(),
            parameters: vec![RawExecutionParameterData {
                id: "target".into(),
                value: RawParameterValueData::Target("core-image-minimal".into()),
            }],
            additional_arguments: vec!["--dry-run".into()],
            interaction: RawInteractionData::NoninteractiveJob,
            safety: RawSafetyData::Build,
            capability_generation: 7,
            build_directory: "/work/build".into(),
            preview_digest: "ab".repeat(32),
        }
    }

    fn raw_execution_chunk_fixture() -> RawOutputChunkData {
        RawOutputChunkData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            stream_id: "raw-stream:stdout-1".into(),
            stream: RawOutputStreamData::Stdout,
            sequence: 1,
            text: "héllo\n".into(),
            truncated_bytes: 2,
            dropped_lines: 1,
        }
    }

    fn raw_execution_snapshot_fixture(sequence: u64) -> RawExecutionSnapshotData {
        let chunk = raw_execution_chunk_fixture();
        RawExecutionSnapshotData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            request: raw_execution_request_fixture(),
            phase: RawExecutionPhaseData::Running,
            attachment: RawAttachmentData::Detached,
            owner: Some(RawExecutionOwnerData::Job("raw-job:protocol-1".into())),
            cancellation_requested: false,
            queued_unix_ms: 10,
            started_unix_ms: Some(20),
            elapsed_ms: 30,
            result: None,
            stdout: RawRetainedOutputData {
                stream_id: chunk.stream_id.clone(),
                stream: RawOutputStreamData::Stdout,
                retained_bytes: chunk.text.len() as u64,
                retained_lines: raw_protocol_line_count(&chunk.text) as u64,
                chunks: vec![chunk],
                next_sequence: 2,
                dropped_bytes: 2,
                dropped_lines: 1,
                truncated_chunks: 1,
            },
            stderr: RawRetainedOutputData {
                stream_id: "raw-stream:stderr-1".into(),
                stream: RawOutputStreamData::Stderr,
                chunks: Vec::new(),
                next_sequence: 1,
                retained_bytes: 0,
                retained_lines: 0,
                dropped_bytes: 0,
                dropped_lines: 0,
                truncated_chunks: 0,
            },
            sequence,
            generation: sequence,
        }
    }

    #[test]
    fn raw_execution_protocol_round_trips_request_event_chunk_snapshot_and_result() {
        let request = raw_execution_request_fixture();
        request.validate().unwrap();
        let request_round_trip: RawExecutionRequestData =
            serde_json::from_slice(&serde_json::to_vec(&request).unwrap()).unwrap();
        assert_eq!(request_round_trip, request);

        let chunk = raw_execution_chunk_fixture();
        chunk.validate().unwrap();
        let chunk_round_trip: RawOutputChunkData =
            serde_json::from_slice(&serde_json::to_vec(&chunk).unwrap()).unwrap();
        assert_eq!(chunk_round_trip, chunk);

        let result = RawExecutionResultData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            outcome: RawExecutionOutcomeData::Cancelled,
            exit_code: None,
            message: Some("cancelled by client".into()),
            elapsed_ms: 40,
            durable_reference: Some("raw-durable:history-1".into()),
        };
        result.validate().unwrap();
        let result_round_trip: RawExecutionResultData =
            serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
        assert_eq!(result_round_trip, result);

        let event = RawExecutionEventData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            request_id: request.request_id.clone(),
            sequence: 2,
            generation: 9,
            event: RawExecutionEventKindData::Finished { result },
        };
        event.validate().unwrap();
        let event_round_trip: RawExecutionEventData =
            serde_json::from_slice(&serde_json::to_vec(&event).unwrap()).unwrap();
        assert_eq!(event_round_trip, event);

        let snapshot = raw_execution_snapshot_fixture(4);
        snapshot.validate().unwrap();
        let snapshot_round_trip: RawExecutionSnapshotData =
            serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
        assert_eq!(snapshot_round_trip, snapshot);

        let command = ClientMessage::Command(CommandRequest {
            request_id: RequestId(4),
            expected_generation: Some(8),
            command: DaemonCommand::StartRaw { request },
        });
        assert_eq!(
            decode_frame::<ClientMessage>(&encode_frame(&command).unwrap()).unwrap(),
            command
        );
    }

    #[test]
    fn raw_job_start_and_cancel_preserve_request_correlation() {
        let request = raw_execution_request_fixture();
        let request_id = request.request_id.clone();
        let start = ClientMessage::Command(CommandRequest {
            request_id: RequestId(41),
            expected_generation: Some(9),
            command: DaemonCommand::StartRaw { request },
        });
        let decoded = decode_frame::<ClientMessage>(&encode_frame(&start).unwrap()).unwrap();
        assert_eq!(decoded, start);

        let cancel = ClientMessage::Command(CommandRequest {
            request_id: RequestId(42),
            expected_generation: Some(10),
            command: DaemonCommand::CancelRaw { request_id },
        });
        assert_eq!(
            decode_frame::<ClientMessage>(&encode_frame(&cancel).unwrap()).unwrap(),
            cancel
        );
    }

    #[test]
    fn raw_pty_start_preserves_only_confirmed_intent_and_dimensions() {
        let mut request = raw_execution_request_fixture();
        request.interaction = RawInteractionData::InteractivePty;
        let start = ClientMessage::Command(CommandRequest {
            request_id: RequestId(43),
            expected_generation: Some(11),
            command: DaemonCommand::StartRawPty {
                request,
                dimensions: TerminalDimensions {
                    columns: 132,
                    rows: 43,
                },
            },
        });
        assert_eq!(
            decode_frame::<ClientMessage>(&encode_frame(&start).unwrap()).unwrap(),
            start
        );
    }

    #[test]
    fn raw_output_attachment_command_preserves_request_correlation() {
        let command = ClientMessage::Command(CommandRequest {
            request_id: RequestId(44),
            expected_generation: Some(12),
            command: DaemonCommand::SetRawAttachment {
                request_id: "raw-request:protocol-output".into(),
                attached: false,
            },
        });
        assert_eq!(
            decode_frame::<ClientMessage>(&encode_frame(&command).unwrap()).unwrap(),
            command
        );
    }

    #[test]
    fn raw_execution_protocol_rejects_unknown_cross_kind_and_unicode_byte_overflow() {
        let mut request = raw_execution_request_fixture();
        request.request_id = "raw-job:protocol-1".into();
        assert_eq!(
            request.validate(),
            Err(RawExecutionProtocolError::InvalidIdentity("request"))
        );

        let mut request = raw_execution_request_fixture();
        request.interaction = serde_json::from_str("\"future_mode\"").unwrap();
        assert_eq!(
            request.validate(),
            Err(RawExecutionProtocolError::UnknownRequiredVariant)
        );

        let mut request = raw_execution_request_fixture();
        request.additional_arguments = vec!["界".repeat(MAX_RAW_EXECUTION_ARGUMENT_BYTES / 3 + 1)];
        assert_eq!(
            request.validate(),
            Err(RawExecutionProtocolError::InvalidArguments)
        );

        let future: RawExecutionEventKindData =
            serde_json::from_str(r#"{"type":"future_required"}"#).unwrap();
        assert_eq!(future, RawExecutionEventKindData::Unknown);
        let event = RawExecutionEventData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            request_id: "raw-request:future".into(),
            sequence: 1,
            generation: 1,
            event: future,
        };
        assert_eq!(
            event.validate(),
            Err(RawExecutionProtocolError::UnknownRequiredVariant)
        );
    }

    #[test]
    fn raw_execution_protocol_snapshot_journal_replaces_newer_and_rejects_stale() {
        let mut base = daemon_snapshot_fixture();
        base.raw_executions = vec![raw_execution_snapshot_fixture(1)];
        let mut journal =
            DaemonSnapshotJournal::new(base, DaemonSnapshotLimits::default()).unwrap();
        journal
            .publish(DaemonEvent::RawExecutionChanged(Box::new(
                raw_execution_snapshot_fixture(2),
            )))
            .unwrap();
        assert_eq!(journal.snapshot().raw_executions[0].sequence, 2);
        let before = journal.snapshot().clone();
        assert!(matches!(
            journal.publish(DaemonEvent::RawExecutionChanged(Box::new(
                raw_execution_snapshot_fixture(2)
            ))),
            Err(DaemonSnapshotError::StaleRawExecution { .. })
        ));
        assert_eq!(journal.snapshot(), &before);
    }

    fn terminal_raw_snapshot(
        sequence: u64,
        outcome: RawExecutionOutcomeData,
    ) -> RawExecutionSnapshotData {
        let mut snapshot = raw_execution_snapshot_fixture(sequence);
        snapshot.phase = RawExecutionPhaseData::Terminal(outcome);
        snapshot.cancellation_requested = outcome == RawExecutionOutcomeData::Cancelled;
        snapshot.elapsed_ms = 50 + sequence;
        snapshot.result = Some(RawExecutionResultData {
            schema_version: RAW_EXECUTION_SCHEMA_VERSION,
            outcome,
            exit_code: match outcome {
                RawExecutionOutcomeData::Succeeded => Some(0),
                RawExecutionOutcomeData::Failed => Some(2),
                RawExecutionOutcomeData::Cancelled | RawExecutionOutcomeData::Lost => None,
                RawExecutionOutcomeData::Unknown => unreachable!(),
            },
            message: Some("must not be retained".into()),
            elapsed_ms: snapshot.elapsed_ms,
            durable_reference: Some("raw-durable:protocol-history".into()),
        });
        snapshot
    }

    #[test]
    fn raw_history_records_all_terminal_outcomes_without_live_authority() {
        assert_eq!(
            RawHistoryRecordData::from_terminal(&raw_execution_snapshot_fixture(1)),
            Err(RawExecutionProtocolError::HistoryRequiresTerminal)
        );
        for outcome in [
            RawExecutionOutcomeData::Succeeded,
            RawExecutionOutcomeData::Failed,
            RawExecutionOutcomeData::Cancelled,
            RawExecutionOutcomeData::Lost,
        ] {
            let record =
                RawHistoryRecordData::from_terminal(&terminal_raw_snapshot(1, outcome)).unwrap();
            assert_eq!(record.outcome, outcome);
            let json = serde_json::to_string(&record).unwrap();
            for prohibited in [
                "raw-job:",
                "owner",
                "stdout",
                "stderr",
                "capability_generation",
                "build_directory",
                "preview_digest",
                "additional_arguments",
                "must not be retained",
            ] {
                assert!(!json.contains(prohibited), "retained {prohibited}: {json}");
            }
        }
        let mut sensitive = terminal_raw_snapshot(1, RawExecutionOutcomeData::Succeeded);
        sensitive.request.parameters.extend([
            RawExecutionParameterData {
                id: "free-text".into(),
                value: RawParameterValueData::Text("token-like-value".into()),
            },
            RawExecutionParameterData {
                id: "temporary-file".into(),
                value: RawParameterValueData::File("/tmp/private-input".into()),
            },
        ]);
        let sanitized = RawHistoryRecordData::from_terminal(&sensitive).unwrap();
        assert!(sanitized.parameters.iter().all(|parameter| !matches!(
            &parameter.value,
            RawParameterValueData::Text(_) | RawParameterValueData::File(_)
        )));
    }

    #[test]
    fn raw_history_journal_updates_duplicate_request_idempotently_newest_first() {
        let base = daemon_snapshot_fixture();
        let mut journal =
            DaemonSnapshotJournal::new(base, DaemonSnapshotLimits::default()).unwrap();
        journal
            .publish(DaemonEvent::RawExecutionChanged(Box::new(
                terminal_raw_snapshot(1, RawExecutionOutcomeData::Failed),
            )))
            .unwrap();
        journal
            .publish(DaemonEvent::RawExecutionChanged(Box::new(
                terminal_raw_snapshot(2, RawExecutionOutcomeData::Succeeded),
            )))
            .unwrap();
        assert_eq!(journal.snapshot().raw_history.len(), 1);
        assert_eq!(
            journal.snapshot().raw_history[0].outcome,
            RawExecutionOutcomeData::Succeeded
        );

        let mut second = terminal_raw_snapshot(1, RawExecutionOutcomeData::Lost);
        second.request.request_id = "raw-request:second".into();
        second.result.as_mut().unwrap().durable_reference = None;
        journal
            .publish(DaemonEvent::RawExecutionChanged(Box::new(second)))
            .unwrap();
        assert_eq!(journal.snapshot().raw_history.len(), 2);
        assert!(
            journal
                .snapshot()
                .raw_history
                .windows(2)
                .all(|pair| { pair[0].ended_unix_ms >= pair[1].ended_unix_ms })
        );
    }

    #[test]
    fn raw_history_rejects_unknown_schema_count_aggregate_and_order() {
        let record = RawHistoryRecordData::from_terminal(&terminal_raw_snapshot(
            1,
            RawExecutionOutcomeData::Succeeded,
        ))
        .unwrap();
        let mut future = record.clone();
        future.schema_version += 1;
        assert!(matches!(
            future.validate(),
            Err(RawExecutionProtocolError::UnsupportedHistorySchema(_))
        ));

        let mut too_many = Vec::new();
        for index in 0..=MAX_RAW_HISTORY_RECORDS {
            let mut item = record.clone();
            item.request_id = format!("raw-request:bounded-{index}");
            too_many.push(item);
        }
        assert_eq!(
            validate_raw_history_records(&too_many),
            Err(RawExecutionProtocolError::TooManyHistoryRecords)
        );

        let mut oversized = Vec::new();
        for index in 0..MAX_RAW_HISTORY_RECORDS {
            let mut item = record.clone();
            item.request_id = format!("raw-request:large-{index}");
            item.parameters = vec![RawExecutionParameterData {
                id: "text".into(),
                value: RawParameterValueData::Text("x".repeat(MAX_RAW_EXECUTION_PARAMETER_BYTES)),
            }];
            oversized.push(item);
        }
        assert_eq!(
            validate_raw_history_records(&oversized),
            Err(RawExecutionProtocolError::HistoryTooLarge)
        );

        let mut misordered = vec![record.clone(), record];
        misordered[0].request_id = "raw-request:older".into();
        misordered[0].ended_unix_ms -= 1;
        misordered[1].request_id = "raw-request:newer".into();
        assert_eq!(
            validate_raw_history_records(&misordered),
            Err(RawExecutionProtocolError::InvalidHistoryOrder)
        );
    }

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

    #[test]
    fn image_console_pty_kinds_round_trip_without_collapsing_identity() {
        for kind in [PtyKind::QemuConsole, PtyKind::SshConsole] {
            let summary = PtySessionSummary {
                id: PtySessionId(41),
                name: "image console".into(),
                kind,
                cwd: "/build".into(),
                lifecycle: LifecycleState::Running,
                dimensions: TerminalDimensions {
                    columns: 120,
                    rows: 40,
                },
                writer: None,
                writer_epoch: 0,
                viewers: 1,
                exit_code: None,
                restartable: true,
            };
            let encoded = serde_json::to_vec(&summary).unwrap();
            let decoded: PtySessionSummary = serde_json::from_slice(&encoded).unwrap();
            assert_eq!(decoded, summary);
            let text = String::from_utf8(encoded).unwrap();
            assert!(
                text.contains(match kind {
                    PtyKind::QemuConsole => "qemu_console",
                    PtyKind::SshConsole => "ssh_console",
                    _ => unreachable!(),
                }),
                "{text}"
            );
        }
    }
}
