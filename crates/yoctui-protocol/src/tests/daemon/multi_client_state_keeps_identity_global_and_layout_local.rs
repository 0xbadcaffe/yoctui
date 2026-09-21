use super::*;

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
