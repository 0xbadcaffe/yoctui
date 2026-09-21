use super::*;

#[test]
fn daemon_state_partition_fails_closed_when_revision_space_is_exhausted() {
    let mut revision = DaemonRevision {
        instance_id: DaemonModelInstanceId([0; 16]),
        sequence: u64::MAX,
        generation: 0,
    };
    assert_eq!(revision.advance(), Err(DaemonStateError::RevisionExhausted));
}
