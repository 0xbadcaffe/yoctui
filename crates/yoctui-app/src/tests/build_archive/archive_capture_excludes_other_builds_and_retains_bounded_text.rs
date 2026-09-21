use super::*;

#[test]
fn archive_capture_excludes_other_builds_and_retains_bounded_text() {
    let mut s = snapshot();
    for n in 0..600 {
        s.recent_logs.push(LogRecord {
            source: "bitbake".into(),
            severity: LogSeverity::Info,
            message: format!("line {n} \x1b"),
            unix_ms: n,
            recipe: None,
            task: None,
            path: None,
            build: Some("image".into()),
        });
    }
    let r = capture_saved_build(&s, 600).unwrap();
    assert_eq!(r.logs.len(), 256);
    assert!(
        r.logs
            .iter()
            .all(|l| l.unix_ms >= 100 && l.unix_ms <= 500 && !l.message.contains('\x1b'))
    );
    assert_eq!(r.outcome, SavedBuildOutcome::Succeeded);
    assert!(r.limitations[0].contains("not a complete"));
    s.jobs[0].id = JobId(2);
    assert_ne!(capture_saved_build(&s, 600).unwrap().id, r.id);
    s.build_events.clear();
    assert!(capture_saved_build(&s, 600).unwrap().logs.is_empty());
}
