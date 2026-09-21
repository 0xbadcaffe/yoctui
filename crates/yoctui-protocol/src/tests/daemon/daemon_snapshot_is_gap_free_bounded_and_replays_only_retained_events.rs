use super::*;

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
