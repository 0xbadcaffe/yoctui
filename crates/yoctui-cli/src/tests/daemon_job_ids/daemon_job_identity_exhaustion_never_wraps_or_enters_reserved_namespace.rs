use super::*;

#[test]
fn daemon_job_identity_exhaustion_never_wraps_or_enters_reserved_namespace() {
    let ids = DaemonJobIds::from_snapshot(&snapshot(&[JOB_ID_LIMIT - 2]));
    assert_eq!(ids.allocate(), Ok(JobId(JOB_ID_LIMIT - 1)));
    for _ in 0..3 {
        assert_eq!(ids.allocate(), Err("daemon job ID space exhausted"));
    }
    let exhausted = DaemonJobIds::from_snapshot(&snapshot(&[JOB_ID_LIMIT - 1]));
    assert!(exhausted.allocate().is_err());
    assert_eq!(
        DaemonJobIds::from_snapshot(&snapshot(&[u64::MAX])).allocate(),
        Ok(JobId(1))
    );
}
